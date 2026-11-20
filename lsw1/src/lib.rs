//! LEGO Star Wars: The Complete Saga... no, LEGO Star Wars 1 (GameCube) autosplitter
//! for Dolphin, ported from the original ASL. Works on Linux (`dolphin-emu`) and Windows.

#![no_std]

use asr::{
    future::{next_tick, retry},
    settings::Gui,
    string::ArrayCString,
    timer::{self, TimerState},
    Address, Endian, FromEndian, MemoryRangeFlags, Process,
};

asr::async_main!(stable);
asr::panic_handler!();

const STATUS: u32 = 0x802E1CB8;
const GAMESTATE: u32 = 0x804093B0;
const CUTSCENE: u32 = 0x801BB45D;
const DOOR: u32 = 0x801DC964;
/// The level-table index of the loaded level, copied from `GAMESTATE` when its load
/// starts (the store at 0x800A1724 in the decomp's `game/main.s`; 124 is the title).
const LEVEL: u32 = 0x804093AC;
/// Set to 1 only by choosing New Game on the title menu, on the same frame the hub
/// is requested, and cleared once the new-game intro ends. Loading a save or
/// returning to the hub from a level leave it at 0.
const NEW_GAME: u32 = 0x80409118;

const MEM1_SIZE: u64 = 0x2000000;

/// `gamestate` is the requested scene (-1 when none); the title screen is 124.
const TITLE: i32 = 124;
/// The NG+ wrong warps leave Ruin through this door (to the Episode II and Episode I
/// endings), and only they clear it straight to nothing.
const RUIN_WARP_DOOR: &[u8] = b"door_to_archive";

const PROCESS_NAMES: &[&str] = &[
    "dolphin-emu",
    "dolphin-emu-nogui",
    "Dolphin.exe",
    "DolphinQt2.exe",
    "DolphinWx.exe",
    "DolphinNoGUI.exe",
];

#[derive(Gui)]
struct Settings {
    /// Start on New Game
    #[default = true]
    newgame: bool,
    /// End on Darth Maul Outro (Any%)
    #[default = true]
    maul: bool,
    /// Split on every room (Any%)
    #[default = false]
    room: bool,
    /// End on Darth Vader Outro (All Minikits)
    #[default = false]
    vader: bool,
    /// End on A New Hope Outro (100% NSR)
    #[default = false]
    anh: bool,
    /// NG+: reset on New Game instead of the title screen
    #[default = false]
    ngplus: bool,
    /// Split on Ruin wrong warps (NG+: to the Episode I and II Endings)
    #[default = false]
    ruinwarp: bool,
}

/// Story chapters as level-table index ranges (the table at 0x801D68EC, 0xA8 bytes per
/// level): each runs from its `_Intro` to its `_Status` level. Room splits re-arm
/// whenever the loaded level moves to another chapter (or out of them, to the hub,
/// an ending or the title).
const CHAPTERS: &[(i32, i32)] = &[
    (NEGOTIATIONS, 6),
    (INVASION, 13),
    (ESCAPE, 22),
    (PODRACE, 32),
    (THEED, 43),
    (MAUL, 50),
    (KAMINO, 61),
    (FACTORY, 71),
    (72, 75), // Jedi Battle
    (GUNSHIP, 78),
    (DOOKU, 83),
    (86, 87), // Dogfight
    (CHANCELLOR, 97),
    (98, 99), // Grievous
    (KASHYYYK, 104),
    (RUIN, 110),
    (VADER, 115),
    (119, 122), // Bonus
];
const NEGOTIATIONS: i32 = 0;
const INVASION: i32 = 7;
const ESCAPE: i32 = 14;
const PODRACE: i32 = 23;
const THEED: i32 = 33;
const MAUL: i32 = 44;
const KAMINO: i32 = 52;
const FACTORY: i32 = 62;
const GUNSHIP: i32 = 76;
const DOOKU: i32 = 79;
const CHANCELLOR: i32 = 88;
const KASHYYYK: i32 = 100;
const RUIN: i32 = 105;
const VADER: i32 = 111;

/// The chapter (its first level index) a level belongs to.
fn chapter_of(level: i32) -> Option<i32> {
    CHAPTERS
        .iter()
        .find(|&&(first, last)| (first..=last).contains(&level))
        .map(|&(first, _)| first)
}

#[derive(Clone, Copy, Debug)]
enum Trigger {
    /// The door string changes to this door, in the room's chapter.
    Enter(&'static [u8]),
    /// The door string changes away from this door, in the room's chapter.
    Leave(&'static [u8]),
    /// This level (a level-table index) loads. For rooms reached by more than one
    /// door or wrong warp, or whose door never shows up.
    Level(i32),
}

struct Room {
    chapter: i32,
    trigger: Trigger,
}

const fn enter(chapter: i32, door: &'static [u8]) -> Room {
    Room {
        chapter,
        trigger: Trigger::Enter(door),
    }
}
const fn leave(chapter: i32, door: &'static [u8]) -> Room {
    Room {
        chapter,
        trigger: Trigger::Leave(door),
    }
}
const fn level(chapter: i32, index: i32) -> Room {
    Room {
        chapter,
        trigger: Trigger::Level(index),
    }
}

/// Room splits (Any%), each taken once per chapter visit. Door names repeat between
/// levels (`door_3`, `door_8`, ...), so door rooms only count in their own chapter.
const ROOMS: &[Room] = &[
    enter(NEGOTIATIONS, b"door_to_hangar"),
    enter(INVASION, b"door_into_ruins1"),
    enter(INVASION, b"door_into_ruins2"),
    leave(INVASION, b"door_into_ruins2"), // wrong warp
    enter(INVASION, b"door_swamp"),       // normal
    enter(ESCAPE, b"RescueA_RescueB"),
    enter(ESCAPE, b"RescueB_RescueC"),
    enter(ESCAPE, b"RescueC_RescueE"),
    enter(THEED, b"retakeA_retakeB"),
    enter(THEED, b"retakeB_retakeD"),
    enter(THEED, b"retakeD_retakeE"),
    enter(THEED, b"retakeE_retakeF"),
    enter(THEED, b"Palace_Hangar"),
    enter(KAMINO, b"door1in"),
    enter(KAMINO, b"door10out"),
    leave(KAMINO, b"door10out"),
    enter(KAMINO, b"door2corr"),
    enter(KAMINO, b"door_bossout"),
    enter(FACTORY, b"door_4"),
    enter(FACTORY, b"door_3in"),
    enter(FACTORY, b"door_5in"),
    enter(FACTORY, b"door_6in"),
    enter(FACTORY, b"door_8out"),
    enter(GUNSHIP, b"GunshipA_mid"),
    enter(GUNSHIP, b"GunshipA_B"),
    enter(DOOKU, b"door_main_in"),
    enter(CHANCELLOR, b"door_grev"),
    // Rooms 3 to 7 (Cruiser_B to Cruiser_F), normal or wrong warp.
    level(CHANCELLOR, 89),
    level(CHANCELLOR, 90),
    level(CHANCELLOR, 91),
    level(CHANCELLOR, 92),
    level(CHANCELLOR, 93),
    enter(KASHYYYK, b"door_to_beach"),
    enter(KASHYYYK, b"door_to_swamp"),
    enter(KASHYYYK, b"door_to_forest"),
    enter(RUIN, b"door_to_inside"),
    level(RUIN, 109), // Temple_C, normal or wrong warp
    enter(RUIN, b"door_to_control"),
    // Vader_B and Vader_C: the first door reads empty and the second has none.
    level(VADER, 113),
    level(VADER, 114),
    enter(VADER, b"door_fight"),
    enter(MAUL, b"door_2"),
    enter(MAUL, b"door_3"),
    // Maul_E, through door_17 or the wrong warp. Backing out of a failed wrong warp
    // only clears door_3 and stays in Maul_D, so it doesn't split.
    level(MAUL, 48),
    enter(MAUL, b"door_8"),
];

// One `RoomSplits::taken` bit per room.
const _: () = assert!(ROOMS.len() <= 64);

/// The podrace lap door: its first two crossings (laps 1 to 2 and 2 to 3) are room
/// splits; the third rolls straight into the outro.
const PODRACE_LAP: &[u8] = b"door_B";
const PODRACE_LAP_SPLITS: u8 = 2;

/// Podrace lap doors. Finishing the race means crossing the third lap door twice,
/// and each crossing briefly raises `status` before the level actually ends. The
/// door reads `door_A`/`door_B` when the lap door is hit and clears to nothing
/// right after, so a `status` rise during either is ignored instead of firing two
/// extra splits.
const PODRACE_DOORS: &[&[u8]] = &[b"door_A", b"door_B"];

#[derive(Clone, Copy, Default)]
struct Watched {
    status: u8,
    gamestate: i32,
    level: i32,
    new_game: u32,
    cutscene: u8,
    door: ArrayCString<16>,
}

struct Emu {
    process: Process,
    ram: Address,
}

impl Emu {
    fn read<T: bytemuck::CheckedBitPattern + FromEndian>(&self, addr: u32) -> Option<T> {
        let off = addr.checked_sub(0x8000_0000)? as u64;
        self.process
            .read::<T>(self.ram + off)
            .ok()
            .map(|v| v.from_endian(Endian::Big))
    }

    fn read_door(&self) -> Option<ArrayCString<16>> {
        let off = (DOOR - 0x8000_0000) as u64;
        self.process.read(self.ram + off).ok()
    }

    fn poll(&self) -> Option<Watched> {
        Some(Watched {
            status: self.read(STATUS)?,
            gamestate: self.read(GAMESTATE)?,
            level: self.read(LEVEL)?,
            new_game: self.read(NEW_GAME)?,
            cutscene: self.read(CUTSCENE)?,
            door: self.read_door()?,
        })
    }
}

/// Finds Dolphin's emulated MEM1. Dolphin exposes it as a 32 MiB rw mapping;
/// bytes 0x1C..0x20 of GameCube memory hold the DVD magic 0xC2339F3D.
fn find_ram(process: &Process) -> Option<Address> {
    process
        .memory_ranges()
        .find(|range| {
            range
                .flags()
                .is_ok_and(|f| f.contains(MemoryRangeFlags::READ | MemoryRangeFlags::WRITE))
                && range.size().is_ok_and(|s| s == MEM1_SIZE)
                && range.address().is_ok_and(|a| {
                    process
                        .read::<u32>(a + 0x1C)
                        .is_ok_and(|m| u32::from_be(m) == 0xC233_9F3D)
                })
        })?
        .address()
        .ok()
}

fn attach() -> Option<Emu> {
    let process = PROCESS_NAMES.iter().find_map(|n| Process::attach(n))?;
    let ram = find_ram(&process)?;
    Some(Emu { process, ram })
}

fn door_is(door: &ArrayCString<16>, name: &[u8]) -> bool {
    door.as_bytes() == name
}

fn door_in(door: &ArrayCString<16>, set: &[&[u8]]) -> bool {
    set.iter().any(|n| door_is(door, n))
}

/// Whether a `status` rise seen between these two states belongs to a podrace lap
/// door instead of the level end: the door is a lap door, or it just cleared.
fn podrace_lap_door(old: &Watched, cur: &Watched) -> bool {
    door_in(&cur.door, PODRACE_DOORS) || (door_in(&old.door, PODRACE_DOORS) && cur.door.is_empty())
}

fn new_game_chosen(old: &Watched, cur: &Watched) -> bool {
    old.new_game == 0 && cur.new_game != 0
}

fn should_start(s: &Settings, old: &Watched, cur: &Watched) -> bool {
    s.newgame && new_game_chosen(old, cur)
}

/// NG+ runs pass through the title screen, so they reset on the next New Game instead.
fn should_reset(s: &Settings, old: &Watched, cur: &Watched) -> bool {
    if s.ngplus {
        new_game_chosen(old, cur)
    } else {
        cur.gamestate == TITLE
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Start,
    Reset,
    /// Reset, then start the next run on the same frame.
    Restart,
    Split,
}

fn action(
    s: &Settings,
    rooms: &mut RoomSplits,
    phase: TimerState,
    old: &Watched,
    cur: &Watched,
) -> Option<Action> {
    if chapter_of(old.level) != chapter_of(cur.level) {
        *rooms = RoomSplits::default();
    }
    let reset = || {
        if should_start(s, old, cur) {
            Action::Restart
        } else {
            Action::Reset
        }
    };
    match phase {
        TimerState::NotRunning => should_start(s, old, cur).then_some(Action::Start),
        TimerState::Running | TimerState::Paused if should_reset(s, old, cur) => Some(reset()),
        TimerState::Running => should_split(s, rooms, old, cur).then_some(Action::Split),
        // A finished NG+ run also restarts on the next New Game.
        TimerState::Ended if s.ngplus && should_reset(s, old, cur) => Some(reset()),
        _ => None,
    }
}

/// Room splits already taken in this chapter visit.
#[derive(Default)]
struct RoomSplits {
    /// One bit per `ROOMS` entry.
    taken: u64,
    /// Podrace lap door crossings.
    laps: u8,
}

fn room_reached(room: &Room, old: &Watched, cur: &Watched) -> bool {
    // The door changes before the next room's level loads; `gamestate` covers a door
    // that requests a level in the same chapter on that frame.
    let here = chapter_of(cur.level) == Some(room.chapter)
        || chapter_of(cur.gamestate) == Some(room.chapter);
    let moved = old.door != cur.door;
    match room.trigger {
        Trigger::Enter(door) => here && moved && door_is(&cur.door, door),
        Trigger::Leave(door) => here && moved && door_is(&old.door, door),
        Trigger::Level(level) => cur.level == level && old.level != level,
    }
}

fn room_split(rooms: &mut RoomSplits, old: &Watched, cur: &Watched) -> bool {
    if chapter_of(cur.level) == Some(PODRACE)
        && old.door != cur.door
        && door_is(&cur.door, PODRACE_LAP)
    {
        rooms.laps += 1;
        if rooms.laps <= PODRACE_LAP_SPLITS {
            return true;
        }
    }
    for (i, room) in ROOMS.iter().enumerate() {
        if rooms.taken & (1 << i) == 0 && room_reached(room, old, cur) {
            rooms.taken |= 1 << i;
            return true;
        }
    }
    false
}

fn should_split(s: &Settings, rooms: &mut RoomSplits, old: &Watched, cur: &Watched) -> bool {
    if old.status == 0 && cur.status == 1 && !podrace_lap_door(old, cur) {
        return true;
    }
    if s.room && room_split(rooms, old, cur) {
        return true;
    }
    if s.maul && door_is(&cur.door, b"door_8") && cur.cutscene == 1 {
        return true;
    }
    if s.vader && door_is(&cur.door, b"door_fight") && cur.cutscene == 1 {
        return true;
    }
    if s.anh && door_is(&cur.door, b"door_from_room2") && cur.cutscene == 1 {
        return true;
    }
    if s.ruinwarp && door_is(&old.door, RUIN_WARP_DOOR) && cur.door.as_bytes().is_empty() {
        return true;
    }
    false
}

async fn main() {
    let mut settings = Settings::register();

    loop {
        let emu = retry(attach).await;
        let mut old: Option<Watched> = None;
        let mut rooms = RoomSplits::default();

        while emu.process.is_open() {
            settings.update();

            let Some(cur) = emu.poll() else {
                // RAM went away (emulation stopped); re-scan.
                if find_ram(&emu.process) != Some(emu.ram) {
                    break;
                }
                next_tick().await;
                continue;
            };

            if let Some(old) = old {
                match action(&settings, &mut rooms, timer::state(), &old, &cur) {
                    Some(Action::Start) => timer::start(),
                    Some(Action::Reset) => timer::reset(),
                    Some(Action::Restart) => {
                        timer::reset();
                        timer::start();
                    }
                    Some(Action::Split) => timer::split(),
                    None => {}
                }
            }
            old = Some(cur);

            next_tick().await;
        }

        timer::pause_game_time();
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::{format, string::String, vec::Vec};

    fn door(name: &str) -> ArrayCString<16> {
        let mut bytes = [0u8; 16];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        bytemuck::cast(bytes)
    }

    fn at(name: &str) -> Watched {
        Watched {
            door: door(name),
            ..Watched::default()
        }
    }

    fn settings(room: bool, maul: bool) -> Settings {
        Settings {
            newgame: true,
            ngplus: false,
            maul,
            room,
            vader: false,
            anh: false,
            ruinwarp: false,
        }
    }

    fn names(list: &[&[u8]]) -> Vec<String> {
        let mut v: Vec<String> = list
            .iter()
            .map(|n| String::from_utf8(n.to_vec()).unwrap())
            .collect();
        v.sort();
        v
    }

    /// In a chapter's level, with this door.
    fn inside(level: i32, name: &str) -> Watched {
        Watched {
            level,
            gamestate: -1,
            ..at(name)
        }
    }

    fn quoted_lines<'a>(asl: &'a str, from: &'a str) -> impl Iterator<Item = &'a str> {
        asl.lines()
            .skip_while(move |l| !l.contains(from))
            .skip(1)
            .take_while(|l| !l.trim_start().starts_with("};"))
    }

    /// The chapters and rooms are a hand port of `dolphin.asl`; keep them identical.
    #[test]
    fn rooms_match_dolphin_asl() {
        let asl = include_str!("../dolphin.asl");
        let chapters: Vec<(i32, i32)> = quoted_lines(asl, "var chapters = new int[][] {")
            .map(|l| {
                let nums: Vec<i32> = l
                    .split(['{', '}'])
                    .nth(1)
                    .unwrap()
                    .split(',')
                    .map(|n| n.trim().parse().unwrap())
                    .collect();
                (nums[0], nums[1])
            })
            .collect();
        assert_eq!(chapters, CHAPTERS);

        let rooms: Vec<String> = quoted_lines(asl, "vars.Rooms = new string[] {")
            .map(|l| l.trim().trim_end_matches(',').trim_matches('"').into())
            .collect();
        let expected: Vec<String> = ROOMS
            .iter()
            .map(|r| match r.trigger {
                Trigger::Enter(d) => format!("{} in {}", r.chapter, names(&[d])[0]),
                Trigger::Leave(d) => format!("{} out {}", r.chapter, names(&[d])[0]),
                Trigger::Level(l) => format!("{} level {l}", r.chapter),
            })
            .collect();
        assert_eq!(rooms, expected);

        for r in ROOMS {
            match r.trigger {
                // `read_door` reads 16 bytes, so a longer name could never match.
                Trigger::Enter(d) | Trigger::Leave(d) => assert!(d.len() <= 16),
                Trigger::Level(l) => assert_eq!(chapter_of(l), Some(r.chapter), "{l}"),
            }
        }
        assert!(asl.contains(&format!("vars.PodraceLapSplits = {PODRACE_LAP_SPLITS};")));
        assert!(asl.contains(&format!(
            "if (vars.ChapterOf(current.level) == {PODRACE} && old.door != current.door && current.door == \"{}\")",
            names(&[PODRACE_LAP])[0]
        )));
        assert!(asl.contains("emu.Make<int>(\"level\", 0x804093AC);"));
        assert_eq!(LEVEL, 0x804093AC);
        assert!(asl.contains("if (vars.ChapterOf(current.level) != vars.ChapterOf(old.level)) {"));
    }

    #[test]
    fn door_rooms_only_split_in_their_chapter_on_a_door_change() {
        let on = settings(true, false);
        let negotiations = |d| inside(2, d);
        assert!(
            should_split(
                &on,
                &mut RoomSplits::default(),
                &negotiations("door_x"),
                &negotiations("door_to_hangar")
            ),
            "entering a listed door"
        );
        assert!(
            !should_split(
                &on,
                &mut RoomSplits::default(),
                &negotiations("door_to_hangar"),
                &negotiations("door_to_hangar")
            ),
            "no change, no split"
        );
        assert!(
            should_split(
                &on,
                &mut RoomSplits::default(),
                &inside(55, "door10out"),
                &inside(55, "door_x")
            ),
            "leaving a listed door"
        );
        assert!(
            !should_split(
                &on,
                &mut RoomSplits::default(),
                &inside(66, "door_x"),
                &inside(66, "door_3")
            ),
            "Maul's door_3 in Factory"
        );
        assert!(
            should_split(
                &on,
                &mut RoomSplits::default(),
                &inside(123, "door_x"),
                &Watched {
                    gamestate: 2,
                    ..inside(123, "door_to_hangar")
                }
            ),
            "a door that requests a level of its chapter"
        );
        assert!(
            !should_split(
                &settings(false, false),
                &mut RoomSplits::default(),
                &negotiations("door_x"),
                &negotiations("door_to_hangar")
            ),
            "setting off"
        );
    }

    #[test]
    fn level_rooms_split_when_the_level_loads() {
        let on = settings(true, false);
        let mut rooms = RoomSplits::default();
        let mut step = |old: &Watched, cur: &Watched| {
            action(&on, &mut rooms, TimerState::Running, old, cur) == Some(Action::Split)
        };
        assert!(
            step(&inside(46, "door_x"), &inside(46, "door_3")),
            "into room 3"
        );
        assert!(
            !step(&inside(47, "door_3"), &inside(47, "")),
            "backing out of a failed wrong warp only clears door_3"
        );
        assert!(step(&inside(47, ""), &inside(48, "")), "Maul_E loads");
        assert!(!step(&inside(48, ""), &inside(48, "")));
        assert!(!step(&inside(48, ""), &inside(47, "")));
        assert!(
            !step(&inside(47, ""), &inside(48, "")),
            "only once per visit"
        );
    }

    #[test]
    fn rooms_split_once_per_chapter_visit() {
        let on = settings(true, false);
        let mut rooms = RoomSplits::default();
        let mut step = |old: &Watched, cur: &Watched| {
            action(&on, &mut rooms, TimerState::Running, old, cur) == Some(Action::Split)
        };
        let cruiser = |d| inside(88, d);
        assert!(
            step(&cruiser("door_x"), &cruiser("door_grev")),
            "first time through"
        );
        assert!(
            !step(&cruiser("door_x"), &cruiser("door_grev")),
            "backtracking through it again"
        );
        assert!(!step(&cruiser("door_x"), &cruiser("door_x")));
        assert!(!step(&cruiser(""), &inside(123, "")), "back in the hub");
        assert!(
            step(&inside(88, "door_x"), &cruiser("door_grev")),
            "re-armed by leaving the chapter"
        );
    }

    #[test]
    fn podrace_splits_the_first_two_laps() {
        let on = settings(true, false);
        let mut rooms = RoomSplits::default();
        let mut laps = Vec::new();
        for _ in 0..3 {
            laps.push(should_split(
                &on,
                &mut rooms,
                &inside(27, ""),
                &inside(27, "door_B"),
            ));
        }
        assert_eq!(laps, [true, true, false]);
        assert!(
            !should_split(
                &on,
                &mut RoomSplits::default(),
                &inside(66, ""),
                &inside(66, "door_B")
            ),
            "outside the podrace"
        );
    }

    /// The podrace doors where `status` rises are a hand port of `dolphin.asl` too.
    #[test]
    fn podrace_doors_match_dolphin_asl() {
        let line = include_str!("../dolphin.asl")
            .lines()
            .find(|l| l.contains("old.status == 0") && l.contains("current.status == 1"))
            .expect("the status split line in dolphin.asl");
        let mut expected: Vec<String> = line
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        expected.sort();
        expected.dedup();
        assert_eq!(names(PODRACE_DOORS), expected);
    }

    #[test]
    fn podrace_lap_doors_do_not_fire_the_status_split() {
        let off = settings(false, false);
        let idle = |door: &str| Watched {
            status: 0,
            ..at(door)
        };
        let finished = |door: &str| Watched {
            status: 1,
            ..at(door)
        };
        for podrace in ["door_A", "door_B"] {
            assert!(
                !should_split(
                    &off,
                    &mut RoomSplits::default(),
                    &idle(podrace),
                    &finished(podrace)
                ),
                "{podrace}: crossing the third lap door is not the level end"
            );
            assert!(
                !should_split(
                    &off,
                    &mut RoomSplits::default(),
                    &idle(podrace),
                    &finished("")
                ),
                "{podrace}: clearing the door right after the lap door is not the level end either"
            );
        }
        assert!(
            should_split(&off, &mut RoomSplits::default(), &idle(""), &finished("")),
            "the real level end still splits once the door is clear"
        );
        assert!(
            should_split(
                &off,
                &mut RoomSplits::default(),
                &at("door_x"),
                &finished("door_y")
            ),
            "a status rise elsewhere still splits"
        );
    }

    #[test]
    fn starts_only_when_new_game_is_chosen() {
        let s = settings(false, false);
        let state = |new_game, gamestate| Watched {
            new_game,
            gamestate,
            ..Watched::default()
        };
        assert!(
            should_start(&s, &state(0, -1), &state(1, 123)),
            "New Game on the title menu"
        );
        assert!(
            !should_start(&s, &state(0, -1), &state(0, 123)),
            "loading a save or quitting a level into the hub"
        );
        assert!(
            !should_start(&s, &state(1, 123), &state(1, -1)),
            "the flag staying up during the intro"
        );
        let mut off = settings(false, false);
        off.newgame = false;
        assert!(
            !should_start(&off, &state(0, -1), &state(1, 123)),
            "setting off"
        );
    }

    #[test]
    fn ngplus_resets_on_new_game_not_the_title() {
        let state = |new_game, gamestate| Watched {
            new_game,
            gamestate,
            ..Watched::default()
        };
        let (title, new_game) = (state(0, TITLE), state(1, 123));
        let any = settings(false, false);
        let mut ng = settings(false, false);
        ng.ngplus = true;

        assert_eq!(
            action(
                &any,
                &mut RoomSplits::default(),
                TimerState::Running,
                &state(0, -1),
                &title
            ),
            Some(Action::Reset),
            "other categories reset on the title screen"
        );
        assert_eq!(
            action(
                &ng,
                &mut RoomSplits::default(),
                TimerState::Running,
                &state(0, -1),
                &title
            ),
            None,
            "NG+ keeps running through the title screen"
        );
        for phase in [TimerState::Running, TimerState::Paused, TimerState::Ended] {
            assert_eq!(
                action(&ng, &mut RoomSplits::default(), phase, &title, &new_game),
                Some(Action::Restart),
                "NG+ New Game restarts from {phase:?}"
            );
        }
        assert_eq!(
            action(
                &any,
                &mut RoomSplits::default(),
                TimerState::Ended,
                &title,
                &new_game
            ),
            None,
            "other categories leave a finished run alone"
        );
        assert_eq!(
            action(
                &ng,
                &mut RoomSplits::default(),
                TimerState::NotRunning,
                &title,
                &new_game
            ),
            Some(Action::Start)
        );
        ng.newgame = false;
        assert_eq!(
            action(
                &ng,
                &mut RoomSplits::default(),
                TimerState::Running,
                &title,
                &new_game
            ),
            Some(Action::Reset),
            "without auto start, New Game only resets"
        );
    }

    #[test]
    fn ends_on_the_ruin_wrong_warp() {
        let mut s = settings(false, false);
        s.ruinwarp = true;
        assert!(
            should_split(
                &s,
                &mut RoomSplits::default(),
                &at("door_to_archive"),
                &at("")
            ),
            "door_to_archive clearing straight to nothing"
        );
        assert!(
            !should_split(
                &s,
                &mut RoomSplits::default(),
                &at("door_to_archive"),
                &at("door_x")
            ),
            "door_to_archive changing to another door"
        );
        assert!(
            !should_split(&s, &mut RoomSplits::default(), &at(""), &at("")),
            "only once"
        );
        s.ruinwarp = false;
        assert!(
            !should_split(
                &s,
                &mut RoomSplits::default(),
                &at("door_to_archive"),
                &at("")
            ),
            "setting off"
        );
    }

    /// The start and reset conditions are a hand port of `dolphin.asl`; keep them in step.
    #[test]
    fn start_matches_dolphin_asl() {
        let asl = include_str!("../dolphin.asl");
        assert!(asl.contains(&format!("\"newgame\", 0x{NEW_GAME:X}")));
        assert!(asl.contains("settings.Add(\"ngplus\", false,"));
        assert!(asl.contains(
            "if (settings[\"ngplus\"]) return old.newgame == 0 && current.newgame != 0;"
        ));
        assert!(asl.contains("old.newgame == 0 && current.newgame != 0) return true;"));
        assert!(
            asl.contains(&format!("return current.gamestate == {TITLE};")),
            "reset"
        );
        assert!(asl.contains(
            "settings[\"ruinwarp\"] && old.door == \"door_to_archive\" && current.door == \"\")"
        ));
    }

    #[test]
    fn status_and_outro_splits() {
        let off = settings(false, false);
        let (idle, finished) = (
            Watched {
                status: 0,
                ..at("x")
            },
            Watched {
                status: 1,
                ..at("x")
            },
        );
        assert!(
            should_split(&off, &mut RoomSplits::default(), &idle, &finished),
            "status 0 -> 1 always splits"
        );
        assert!(!should_split(
            &off,
            &mut RoomSplits::default(),
            &finished,
            &finished
        ));

        let outro = Watched {
            cutscene: 1,
            ..at("door_8")
        };
        assert!(
            should_split(
                &settings(false, true),
                &mut RoomSplits::default(),
                &at("door_8"),
                &outro
            ),
            "Maul outro with the setting on"
        );
        assert!(
            !should_split(&off, &mut RoomSplits::default(), &at("door_8"), &outro),
            "Maul outro with the setting off"
        );
        assert!(
            !should_split(
                &settings(false, true),
                &mut RoomSplits::default(),
                &at("door_8"),
                &at("door_8")
            ),
            "door_8 without the cutscene"
        );
    }
}
