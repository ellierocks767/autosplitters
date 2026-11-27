#![no_std]

#[cfg(test)]
extern crate std;

use asr::{
    future::{next_tick, retry},
    settings::{gui::Title, Gui},
    time::Duration,
    timer::{self, TimerState},
    Address, Process,
};
use bytemuck::Pod;

#[cfg(not(test))]
use clock::Instant;
#[cfg(test)]
use std::time::Instant;

/// Monotonic wall-clock time for timing pre-rendered movies, when the game's frame counters stop.
///
/// asr only has an `Instant` on the WASI target, but the workspace builds for `wasm32-unknown-unknown`. LiveSplit
/// One's runtime links WASI preview 1 for any module that imports from it, so this imports the one call asr's
/// WASI `Instant` makes.
#[cfg(not(test))]
mod clock {
    use core::time::Duration;

    #[link(wasm_import_module = "wasi_snapshot_preview1")]
    extern "C" {
        fn clock_time_get(id: u32, precision: u64, time: *mut u64) -> u16;
    }

    const CLOCKID_MONOTONIC: u32 = 1;

    #[derive(Clone, Copy)]
    pub struct Instant(u64);

    impl Instant {
        pub fn now() -> Self {
            let mut nanos = 0;
            // SAFETY: `nanos` is a valid u64 for the call to write; the runtime always provides this import.
            let errno = unsafe { clock_time_get(CLOCKID_MONOTONIC, 1, &mut nanos) };
            assert_eq!(errno, 0, "clock_time_get failed");
            Self(nanos)
        }

        pub fn duration_since(&self, earlier: Self) -> Duration {
            Duration::from_nanos(self.0.saturating_sub(earlier.0))
        }
    }
}

asr::async_main!(stable);
asr::panic_handler!();

const GLOBAL_FRAME_COUNT: u64 = 0x0372_C6C8;
const OUTPUT_FRAME_COUNT: u64 = 0x036B_DF58;
const IN_CUTSCENE: u64 = 0x0372_C55C;
const GAME_STATUS: u64 = 0x0372_2DE4;
const GAME_MODE: u64 = 0x036B_DC7C;
const IN_PRERENDERED_MOVIE: u64 = 0x0368_B0D0;
const CURR_CHARACTER: u64 = 0x0372_2DC0;
const LAST_STORY: u64 = 0x0371_8DB4;
const TIMER_START: u64 = 0x0051_2DF0;
const DEMO_PLAYING: u64 = 0x0372_A2E4;
const BOSS_HEALTH: u64 = 0x0385_8150;
const LEVEL: u64 = 0x0372_2DCC;
const LEVEL_COPY: u64 = 0x0372_2DC4;
const ACT: u64 = 0x0372_2DEC;
const LIVES: u64 = 0x0370_EF34;
const E101_FLAG: u64 = 0x0371_89A8;
const ADVENTURE_DATA: u64 = 0x0371_83F0;
const RM_FLAG: u64 = 0x0371_89A5;
const MK2_VALUE: u64 = 0x0386_C760;
const WALKER_VALUE: u64 = 0x0385_A7ED;
const SELECTED_CHARACTER: u64 = 0x0372_A2FD;
const MUSIC: u64 = 0x0051_2698;
const LW_FLAG: u64 = 0x0371_892A;
const EMBLEM_BYTES: u64 = 0x0372_B5E8;
const EVENT_BYTES: u64 = 0x0371_8888;

const LIGHT_SHOES: u64 = 0x0371_8895;
const CRYSTAL_RING: u64 = 0x0371_8896;
const ANCIENT_LIGHT: u64 = 0x0371_88A7;
const JET_ANKLET: u64 = 0x0371_88D5;
const RHYTHM_BADGE: u64 = 0x0371_88E3;
const SHOVEL_CLAW: u64 = 0x0371_8921;
const FIGHTING_GLOVES: u64 = 0x0371_8922;
const WARRIOR_FEATHER: u64 = 0x0371_895A;
const LONG_HAMMER: u64 = 0x0371_8966;
const LIFE_BELT: u64 = 0x0371_89D8;
const POWER_ROD: u64 = 0x0371_89D9;
const LURE_EC: u64 = 0x0371_89E6;
const LURE_IC: u64 = 0x0371_89E7;
const LURE_SS: u64 = 0x0371_89E8;
const LURE_MR: u64 = 0x0371_89E9;
const JET_BOOSTER: u64 = 0x0371_8991;
const LASER_BLASTER: u64 = 0x0371_8992;

#[derive(Gui)]
struct Settings {
    /// General
    _general: Title,
    /// Auto Reset on Main Menu
    reset: bool,
    /// Splits when entering a story
    char_split: bool,
    /// ...But not when in trial
    char_split_ban_trial: bool,
    /// Don't start when in trial
    char_start_ban_trial: bool,

    /// Stages NOT To Split
    _stages: Title,
    /// Sky Chase Act 1
    #[default = true]
    ignore_sky_chase_1: bool,
    /// Sky Chase Act 2
    #[default = true]
    ignore_sky_chase_2: bool,

    /// Final Boss To Split
    _final_bosses: Title,
    /// Egg Viper
    #[default = true]
    egg_viper: bool,
    /// Egg Walker
    #[default = true]
    egg_walker: bool,
    /// Chaos 6
    #[default = true]
    chaos_6: bool,
    /// ZERO
    #[default = true]
    zero: bool,
    /// E-101 'Beta' mkII
    #[default = true]
    beta_mk2: bool,
    /// Perfect Chaos
    #[default = true]
    perfect_chaos: bool,

    /// Sonic Bosses
    _sonic_bosses: Title,
    /// Chaos 0
    #[default = true]
    zero_sonic: bool,
    /// Egg Hornet
    #[default = true]
    egg_hornet_sonic: bool,
    /// Knuckles
    #[default = true]
    knux_sonic: bool,
    /// Chaos 4
    #[default = true]
    four_sonic: bool,
    /// Gamma
    #[default = true]
    gamma_sonic: bool,
    /// Chaos 6
    #[default = true]
    six_sonic: bool,

    /// Tails Bosses
    _tails_bosses: Title,
    /// Egg Hornet
    #[default = true]
    egg_hornet_tails: bool,
    /// Knuckles
    #[default = true]
    knux_tails: bool,
    /// Chaos 4
    #[default = true]
    four_tails: bool,
    /// Gamma
    #[default = true]
    gamma_tails: bool,

    /// Knuckles Bosses
    _knuckles_bosses: Title,
    /// Chaos 2
    #[default = true]
    two_knuckles: bool,
    /// Sonic
    #[default = true]
    sonic_knuckles: bool,
    /// Chaos 4
    #[default = true]
    four_knuckles: bool,

    /// Gamma Bosses
    _gamma_bosses: Title,
    /// E101 BETA
    e101: bool,
    /// Sonic
    #[default = true]
    sonic_gamma: bool,

    /// Miscellaneous
    _global_misc: Title,
    /// Split between the Acts
    act_split: bool,

    /// Sonic Miscellaneous
    _sonic_misc: Title,
    /// Enter Sewers
    enter_sewer_sonic: bool,
    /// Enter Casino Area (To Red Mountain)
    enter_cas_hub_sonic: bool,
    /// Ship Transformation
    ship_transform: bool,
    /// Splits when going back to the present after Lost World
    back_from_past: bool,
    /// Enter Station Square from the Train Station
    enter_ss: bool,
    /// Enter Mystic Ruins from the Train Station
    enter_mr: bool,

    /// Knuckles Miscellaneous
    _knuckles_misc: Title,
    /// Death Warp Mystic Ruin
    death_mr_knux: bool,
    /// Enter Egg Carrier (Before Sky Deck)
    enter_eg_carrier_knux: bool,

    /// Amy Miscellaneous
    _amy_misc: Title,
    /// Enter Casino Area
    enter_cas_hub_amy: bool,
    /// Enter Jungle to Final Egg
    enter_jungle_to_fe_amy: bool,
    /// Exit Jungle from Final Egg
    exit_jungle_to_ec_amy: bool,
    /// Enter Egg Carrier (Before Zero)
    enter_eg_carrier_amy: bool,

    /// Gamma Miscellaneous
    _gamma_misc: Title,
    /// Death Mystic Ruin (Windy Valley Flag)
    death_wv: bool,
    /// Death Warp (Post Red Mountain)
    death_mr_gamma: bool,
    /// Enter Egg Carrier (Before MKII)
    egg_carrier_mk2: bool,

    /// Super Sonic Miscellaneous
    _super_sonic_misc: Title,
    /// Enter Cave
    enter_cave: bool,
    /// Angel Island Cutscene
    angel_island: bool,
    /// Tikal Cutscene
    tikal: bool,
    /// Death Warp post Tikal Cutscene
    death_ss: bool,
    /// Enter Jungle
    jungle_ss: bool,

    /// Enter Boss
    _boss_enter: Title,
    /// Enter Egg Hornet
    enter_egg_hornet: bool,
    /// Enter Chaos 2
    enter_chaos_2: bool,
    /// Enter Chaos 4
    enter_chaos_4: bool,
    /// Enter Chaos 6
    enter_chaos_6: bool,
    /// Enter E-101 'Beta' mkI
    enter_beta_1: bool,
    /// Enter E-101 'Beta' mkII
    enter_beta_2: bool,

    /// Enter Stage
    _stage_enter: Title,
    /// Enter Emerald Coast
    eec: bool,
    /// Enter Windy Valley
    ewv: bool,
    /// Enter Casino
    ec: bool,
    /// Enter Ice Cap
    eic: bool,
    /// Enter Twinkle Park
    etp: bool,
    /// Enter Speed Highway
    esh: bool,
    /// Enter Red Mountain
    erm: bool,
    /// Enter Sky Deck
    esd: bool,
    /// Enter Lost World
    elw: bool,
    /// Enter Final Egg
    efe: bool,
    /// Enter Hot Shelter
    ehs: bool,

    /// Upgrades
    _upgrades: Title,
    /// Light Speed Shoes
    sonic_powerup1: bool,
    /// Crystal Ring
    sonic_powerup2: bool,
    /// Ancient Light
    sonic_powerup3: bool,
    /// Jet Anklet
    tails_powerup1: bool,
    /// Rhythm Badge
    tails_powerup2: bool,
    /// Shovel Claw
    knux_powerup1: bool,
    /// Fighting Gloves
    knux_powerup2: bool,
    /// Warrior Feather
    amy_powerup1: bool,
    /// Long Hammer
    amy_powerup2: bool,
    /// Power Rod
    big_powerup1: bool,
    /// Life Belt
    big_powerup2: bool,
    /// Lure
    big_powerup3: bool,
    /// Jet Booster
    gamma_powerup1: bool,
    /// Laser Blaster
    gamma_powerup2: bool,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct GameState {
    global_frame_count: i32,
    output_frame_count: i32,
    in_cutscene: i32,
    game_status: u8,
    game_mode: u8,
    in_prerendered_movie: i32,
    curr_character: u8,
    last_story: u8,
    timer_start: u8,
    demo_playing: u8,
    boss_health: u8,
    level: u8,
    level_copy: u8,
    act: u8,
    lives: u8,
    e101_flag: u8,
    adventure_data: u8,
    rm_flag: u8,
    mk2_value: u8,
    walker_value: u8,
    selected_character: u8,
    music: u8,
    lw_flag: u8,
    emblem_bytes: [u8; 17],
    event_bytes: [u8; 353],
    lightshoes: u8,
    crystalring: u8,
    ancientlight: u8,
    jetanklet: u8,
    rhythmbadge: u8,
    shovelclaw: u8,
    fightinggloves: u8,
    warriorfeather: u8,
    longhammer: u8,
    lifebelt: u8,
    powerrod: u8,
    lureec: u8,
    lureic: u8,
    luress: u8,
    luremr: u8,
    jetbooster: u8,
    laserblaster: u8,
}

#[cfg(test)]
impl Default for GameState {
    fn default() -> Self {
        Self {
            global_frame_count: 0,
            output_frame_count: 0,
            in_cutscene: 0,
            game_status: 0,
            game_mode: 0,
            in_prerendered_movie: 0,
            curr_character: 0,
            last_story: 0,
            timer_start: 0,
            demo_playing: 0,
            boss_health: 0,
            level: 0,
            level_copy: 0,
            act: 0,
            lives: 0,
            e101_flag: 0,
            adventure_data: 0,
            rm_flag: 0,
            mk2_value: 0,
            walker_value: 0,
            selected_character: 0,
            music: 0,
            lw_flag: 0,
            emblem_bytes: [0; 17],
            event_bytes: [0; 353],
            lightshoes: 0,
            crystalring: 0,
            ancientlight: 0,
            jetanklet: 0,
            rhythmbadge: 0,
            shovelclaw: 0,
            fightinggloves: 0,
            warriorfeather: 0,
            longhammer: 0,
            lifebelt: 0,
            powerrod: 0,
            lureec: 0,
            lureic: 0,
            luress: 0,
            luremr: 0,
            jetbooster: 0,
            laserblaster: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct RunState {
    total_frames: i64,
    prerendered_movie_rta: Duration,
    credits_counter: i32,
    boss_started: bool,
    delay: i32,
    last_tick: Instant,
}

impl RunState {
    fn new(now: Instant) -> Self {
        Self {
            total_frames: 0,
            prerendered_movie_rta: Duration::ZERO,
            credits_counter: -1,
            boss_started: false,
            delay: 0,
            last_tick: now,
        }
    }

    fn reset_for_start(&mut self, now: Instant) {
        *self = Self::new(now);
    }
}

async fn main() {
    let mut settings = Settings::register();

    loop {
        asr::set_tick_rate(1.0);
        let (process, process_name) = retry(|| {
            Process::attach("sonic.exe")
                .map(|process| (process, "sonic.exe"))
                .or_else(|| {
                    Process::attach("Sonic Adventure DX.exe")
                        .map(|process| (process, "Sonic Adventure DX.exe"))
                })
        })
        .await;

        process
            .until_closes(async {
                asr::set_tick_rate(120.0);
                let module = retry(|| process.get_module_address(process_name)).await;
                let mut run = RunState::new(Instant::now());
                let mut old = retry(|| read_state(&process, module)).await;

                loop {
                    next_tick().await;
                    settings.update();

                    let Ok(current) = read_state(&process, module) else {
                        continue;
                    };

                    update_game_time(&current, &old, &mut run);
                    timer::set_game_time(game_time(&run));
                    timer::pause_game_time();

                    match timer::state() {
                        TimerState::NotRunning if should_start(&settings, &current, &old) => {
                            run.reset_for_start(Instant::now());
                            timer::set_game_time(Duration::ZERO);
                            timer::start();
                            timer::pause_game_time();
                        }
                        TimerState::Running | TimerState::Paused => {
                            if should_reset(&settings, &current, &old) {
                                run.reset_for_start(Instant::now());
                                timer::set_game_time(Duration::ZERO);
                                timer::reset();
                            } else if should_split(&settings, &current, &old, &mut run) {
                                timer::split();
                            }
                        }
                        TimerState::Ended => {}
                        _ => {}
                    }

                    old = current;
                }
            })
            .await;
    }
}

fn read<T: Pod>(process: &Process, module: Address, offset: u64) -> Result<T, asr::Error> {
    process.read::<T>(module + offset)
}

fn read_state(process: &Process, module: Address) -> Result<GameState, asr::Error> {
    Ok(GameState {
        global_frame_count: read(process, module, GLOBAL_FRAME_COUNT)?,
        output_frame_count: read(process, module, OUTPUT_FRAME_COUNT)?,
        in_cutscene: read(process, module, IN_CUTSCENE)?,
        game_status: read(process, module, GAME_STATUS)?,
        game_mode: read(process, module, GAME_MODE)?,
        in_prerendered_movie: read(process, module, IN_PRERENDERED_MOVIE)?,
        curr_character: read(process, module, CURR_CHARACTER)?,
        last_story: read(process, module, LAST_STORY)?,
        timer_start: read(process, module, TIMER_START)?,
        demo_playing: read(process, module, DEMO_PLAYING)?,
        boss_health: read(process, module, BOSS_HEALTH)?,
        level: read(process, module, LEVEL)?,
        level_copy: read(process, module, LEVEL_COPY)?,
        act: read(process, module, ACT)?,
        lives: read(process, module, LIVES)?,
        e101_flag: read(process, module, E101_FLAG)?,
        adventure_data: read(process, module, ADVENTURE_DATA)?,
        rm_flag: read(process, module, RM_FLAG)?,
        mk2_value: read(process, module, MK2_VALUE)?,
        walker_value: read(process, module, WALKER_VALUE)?,
        selected_character: read(process, module, SELECTED_CHARACTER)?,
        music: read(process, module, MUSIC)?,
        lw_flag: read(process, module, LW_FLAG)?,
        emblem_bytes: read(process, module, EMBLEM_BYTES)?,
        event_bytes: read(process, module, EVENT_BYTES)?,
        lightshoes: read(process, module, LIGHT_SHOES)?,
        crystalring: read(process, module, CRYSTAL_RING)?,
        ancientlight: read(process, module, ANCIENT_LIGHT)?,
        jetanklet: read(process, module, JET_ANKLET)?,
        rhythmbadge: read(process, module, RHYTHM_BADGE)?,
        shovelclaw: read(process, module, SHOVEL_CLAW)?,
        fightinggloves: read(process, module, FIGHTING_GLOVES)?,
        warriorfeather: read(process, module, WARRIOR_FEATHER)?,
        longhammer: read(process, module, LONG_HAMMER)?,
        lifebelt: read(process, module, LIFE_BELT)?,
        powerrod: read(process, module, POWER_ROD)?,
        lureec: read(process, module, LURE_EC)?,
        lureic: read(process, module, LURE_IC)?,
        luress: read(process, module, LURE_SS)?,
        luremr: read(process, module, LURE_MR)?,
        jetbooster: read(process, module, JET_BOOSTER)?,
        laserblaster: read(process, module, LASER_BLASTER)?,
    })
}

fn story_enter(current: &GameState, old: &GameState) -> bool {
    current.demo_playing != 1
        && old.game_status == 21
        && current.game_mode != 12
        && current.game_mode != 20
        && (old.game_mode == 12 || old.game_mode == 20)
}

fn should_start(settings: &Settings, current: &GameState, old: &GameState) -> bool {
    should_start_with_trial_setting(settings.char_start_ban_trial, current, old)
}

fn should_reset(settings: &Settings, current: &GameState, old: &GameState) -> bool {
    should_reset_with_setting(settings.reset, current, old)
}

fn should_start_with_trial_setting(
    char_start_ban_trial: bool,
    current: &GameState,
    old: &GameState,
) -> bool {
    let start = story_enter(current, old);
    if char_start_ban_trial {
        start && current.game_mode != 9
    } else {
        start
    }
}

fn should_reset_with_setting(reset: bool, current: &GameState, old: &GameState) -> bool {
    reset && current.game_mode == 12 && old.curr_character != 6 && current.level_copy != 32
}

fn should_split(
    settings: &Settings,
    current: &GameState,
    old: &GameState,
    run: &mut RunState,
) -> bool {
    if settings.char_split {
        let split = story_enter(current, old);
        if split && (!settings.char_split_ban_trial || current.game_mode != 9) {
            return true;
        }
    }

    if (settings.ignore_sky_chase_1 && current.level == 36)
        || (settings.ignore_sky_chase_2 && current.level == 37)
    {
        return false;
    }

    if event_split(settings, current, old) {
        return true;
    }

    if current.in_cutscene != 0
        && old.in_cutscene == 0
        && current.level == 29
        && current.curr_character == 7
    {
        return true;
    }

    if settings.e101 && current.curr_character == 6 && current.e101_flag > old.e101_flag {
        return true;
    }

    if final_boss_split(settings, current, old) || boss_timing_split(settings, current, old, run) {
        return true;
    }

    if upgrade_split(settings, current, old)
        || boss_enter_split(settings, current, old)
        || stage_enter_split(settings, current, old)
        || misc_split(settings, current, old)
    {
        return true;
    }

    emblem_split(current, old, run)
}

fn event_split(settings: &Settings, current: &GameState, old: &GameState) -> bool {
    for i in 0..353 {
        if event_enabled(settings, i)
            && current.game_status != 21
            && current.event_bytes[i] != old.event_bytes[i]
        {
            return true;
        }
    }
    false
}

fn event_enabled(settings: &Settings, index: usize) -> bool {
    match index {
        36 => settings.gamma_sonic,
        37 => settings.knux_sonic,
        48 => settings.zero_sonic,
        49 => settings.four_sonic,
        50 => settings.six_sonic,
        51 => settings.egg_hornet_sonic,
        95 => settings.gamma_tails,
        96 => settings.knux_tails,
        103 => settings.four_tails,
        105 => settings.egg_hornet_tails,
        158 => settings.sonic_knuckles,
        163 => settings.two_knuckles,
        165 => settings.four_knuckles,
        282 => settings.sonic_gamma,
        _ => false,
    }
}

fn final_boss_split(settings: &Settings, current: &GameState, old: &GameState) -> bool {
    if current.level == 25 && current.mk2_value == 255 {
        return settings.beta_mk2
            && current.boss_health == 0
            && current.timer_start == 0
            && old.timer_start == 1;
    }
    if current.level == 19 {
        return settings.perfect_chaos && current.boss_health == 0 && old.boss_health == 2;
    }
    if current.level == 23 {
        return settings.zero && current.boss_health == 0 && old.boss_health == 1;
    }
    if current.level == 21 && current.walker_value == 9 {
        return settings.egg_walker
            && current.boss_health == 0
            && current.timer_start == 0
            && old.timer_start == 1;
    }
    false
}

fn boss_timing_split(
    settings: &Settings,
    current: &GameState,
    old: &GameState,
    run: &mut RunState,
) -> bool {
    let tracked_boss =
        (settings.chaos_6 && current.level == 18) || (settings.egg_viper && current.level == 22);
    if !tracked_boss {
        return false;
    }

    if current.boss_health == 0 && current.timer_start == 0 {
        run.boss_started = true;
    }

    if run.boss_started
        && current.in_cutscene == 0
        && old.in_cutscene == 0
        && current.boss_health == 0
        && current.game_status == 15
        && current.timer_start == 0
        && old.timer_start == 1
    {
        return !(current.level == 18 && current.curr_character == 0);
    }

    false
}

fn upgrade_split(settings: &Settings, current: &GameState, old: &GameState) -> bool {
    current.level != 0
        && ((settings.sonic_powerup1 && current.lightshoes > old.lightshoes)
            || (settings.sonic_powerup2 && current.crystalring > old.crystalring)
            || (settings.sonic_powerup3 && current.ancientlight > old.ancientlight)
            || (settings.tails_powerup1 && current.jetanklet > old.jetanklet)
            || (settings.tails_powerup2 && current.rhythmbadge > old.rhythmbadge)
            || (settings.knux_powerup1 && current.shovelclaw > old.shovelclaw)
            || (settings.knux_powerup2 && current.fightinggloves > old.fightinggloves)
            || (settings.amy_powerup1 && current.warriorfeather > old.warriorfeather)
            || (settings.amy_powerup2 && current.longhammer > old.longhammer)
            || (settings.big_powerup1 && current.powerrod > old.powerrod)
            || (settings.big_powerup2 && current.lifebelt > old.lifebelt)
            || (settings.big_powerup3
                && (current.luress > old.luress
                    || current.lureic != old.lureic
                    || current.lureec != old.lureec
                    || current.luremr != old.luremr))
            || (settings.gamma_powerup1 && current.jetbooster > old.jetbooster)
            || (settings.gamma_powerup2 && current.laserblaster > old.laserblaster))
}

fn boss_enter_split(settings: &Settings, current: &GameState, old: &GameState) -> bool {
    (settings.enter_egg_hornet && current.level == 20 && old.level == 33)
        || (settings.enter_chaos_2 && current.level == 16 && old.level == 26)
        || (settings.enter_chaos_4 && current.level == 17 && old.level == 33)
        || (settings.enter_chaos_6 && current.level == 18 && old.level == 29)
        || (settings.enter_beta_1 && current.level == 24 && old.level == 33)
        || (settings.enter_beta_2 && current.level == 25 && old.level == 29)
}

fn stage_enter_split(settings: &Settings, current: &GameState, old: &GameState) -> bool {
    (settings.eec && current.level == 1 && old.level == 26)
        || (settings.ewv && current.level == 2 && old.level == 33)
        || (settings.ec && current.level == 9 && old.level == 26)
        || (settings.eic && current.level == 8 && old.level == 33)
        || (settings.etp && current.level == 3 && old.level == 26)
        || (settings.esh && current.level == 4 && old.level == 26)
        || (settings.erm && current.level == 5 && old.level == 33)
        || (settings.esd && current.level == 6 && old.level == 29)
        || (settings.elw && current.level == 7 && old.level == 33)
        || (settings.efe && current.level == 10 && old.level == 33)
        || (settings.ehs && current.level == 12 && old.level == 32)
}

fn misc_split(settings: &Settings, current: &GameState, old: &GameState) -> bool {
    if settings.act_split
        && current.level != 0
        && current.level != 15
        && current.level != 22
        && current.level != 26
        && current.level != 29
        && current.level != 32
        && current.level != 33
        && current.level != 37
        && current.act != old.act
    {
        return true;
    }

    if settings.enter_sewer_sonic
        && current.level == 26
        && current.act == 2
        && old.act == 0
        && current.curr_character == 0
    {
        return true;
    }
    if settings.enter_cas_hub_sonic
        && current.level == 26
        && ((current.emblem_bytes[8] & 0x8) >> 3) == 1
        && ((current.emblem_bytes[8] & 0x10) >> 4) == 0
        && current.act == 1
        && (old.act == 3 || old.act == 4)
        && current.curr_character == 0
    {
        return true;
    }
    if settings.ship_transform && current.level == 29 && current.act == 6 && old.act == 3 {
        return true;
    }
    if settings.back_from_past
        && current.level == 33
        && current.act == 2
        && old.level == 34
        && old.act == 2
    {
        return true;
    }
    if settings.enter_ss
        && current.level == 26
        && current.act == 1
        && old.level == 33
        && old.act == 0
    {
        return true;
    }
    if settings.enter_mr
        && current.level == 33
        && current.act == 0
        && old.level == 26
        && old.act == 1
    {
        return true;
    }

    if settings.death_wv
        && current.level == 33
        && current.act == 0
        && current.adventure_data != old.adventure_data
        && current.curr_character == 6
    {
        return true;
    }
    if settings.death_mr_gamma
        && current.level == 33
        && current.act == 1
        && current.lives < old.lives
        && current.curr_character == 6
        && current.adventure_data == 2
    {
        return true;
    }
    if settings.egg_carrier_mk2
        && current.level == 29
        && old.level == 33
        && current.rm_flag == 1
        && current.curr_character == 6
    {
        return true;
    }

    if settings.death_mr_knux
        && current.level == 33
        && current.act == 1
        && current.lw_flag == 1
        && current.lives < old.lives
        && current.curr_character == 3
    {
        return true;
    }
    if settings.enter_eg_carrier_knux
        && current.level == 29
        && old.level == 33
        && current.curr_character == 3
    {
        return true;
    }

    if settings.enter_eg_carrier_amy
        && current.level == 29
        && old.level == 33
        && current.curr_character == 5
    {
        return true;
    }
    if settings.enter_cas_hub_amy
        && ((current.emblem_bytes[0xA] & 0x10) >> 4) == 0
        && current.level == 26
        && current.act == 1
        && old.act == 3
        && current.curr_character == 5
    {
        return true;
    }
    if settings.enter_jungle_to_fe_amy
        && current.level == 33
        && current.act == 2
        && old.act == 0
        && current.curr_character == 5
    {
        return true;
    }
    if settings.exit_jungle_to_ec_amy
        && current.level == 33
        && current.act == 0
        && old.act == 2
        && current.curr_character == 5
    {
        return true;
    }

    current.selected_character == 6
        && ((settings.enter_cave && current.level == 33 && current.act == 1 && old.act == 0)
            || (settings.angel_island
                && current.level == 33
                && current.act == 1
                && current.music == 40
                && old.music != 40)
            || (settings.tikal && current.level == 33 && old.level == 34)
            || (settings.death_ss
                && current.level == 33
                && current.act == 1
                && current.lives < old.lives)
            || (settings.jungle_ss
                && current.level == 33
                && old.level != 0
                && current.act == 2
                && old.act == 0))
}

fn emblem_split(current: &GameState, old: &GameState, run: &mut RunState) -> bool {
    for i in 0..current.emblem_bytes.len() {
        if current.game_mode != 12 && current.emblem_bytes[i] != old.emblem_bytes[i] {
            if current.level < 15 {
                run.delay = current.global_frame_count + 190;
            } else {
                return true;
            }
        }
    }

    if run.delay > 0 && (run.delay - current.global_frame_count).abs() <= 2 {
        run.delay = 0;
        return true;
    }

    false
}

fn update_game_time(current: &GameState, old: &GameState, run: &mut RunState) {
    let now = Instant::now();
    let tick_delta = now.duration_since(run.last_tick);

    if current.in_prerendered_movie != 0 {
        let tick_delta = Duration::nanoseconds_i128(tick_delta.as_nanos() as i128);
        run.prerendered_movie_rta = run.prerendered_movie_rta.saturating_add(tick_delta);
    }

    if current.game_mode == 1
        || current.game_mode == 12
        || current.game_mode == 18
        || current.game_mode == 20
        || current.game_status == 19
        || current.game_status == 16
    {
        run.total_frames += i64::from(current.output_frame_count - old.output_frame_count);
    }

    if current.game_mode == 22 {
        if old.game_mode != 22 {
            run.credits_counter = 3600;
        }

        run.credits_counter -= 1;

        if run.credits_counter == 0 {
            run.total_frames += match current.curr_character {
                0 if current.last_story == 0 => 16_804,
                0 => 17_360,
                2 => 14_657,
                3 => 16_804,
                5 => 18_950,
                6 => 17_545,
                7 => 15_333,
                _ => 0,
            };
        }
    }

    let mut delta_frames = current.global_frame_count - old.global_frame_count;
    if current.in_cutscene != 0 {
        delta_frames *= 2;
    }

    run.total_frames += i64::from(delta_frames);
    run.last_tick = now;
}

fn game_time(run: &RunState) -> Duration {
    Duration::saturating_seconds_f64(run.total_frames as f64 / 60.0)
        .saturating_add(run.prerendered_movie_rta)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn story_select_state() -> GameState {
        GameState {
            game_status: 21,
            game_mode: 12,
            ..GameState::default()
        }
    }

    #[test]
    fn starts_on_story_select_exit() {
        let old = story_select_state();
        let current = GameState {
            game_status: 21,
            game_mode: 0,
            ..old
        };

        assert!(should_start_with_trial_setting(false, &current, &old));
    }

    #[test]
    fn can_block_trial_mode_start() {
        let old = story_select_state();
        let current = GameState {
            game_status: 21,
            game_mode: 9,
            ..old
        };

        assert!(should_start_with_trial_setting(false, &current, &old));
        assert!(!should_start_with_trial_setting(true, &current, &old));
    }

    #[test]
    fn reset_requires_setting_and_main_menu() {
        let old = GameState::default();
        let current = GameState {
            game_mode: 12,
            ..GameState::default()
        };

        assert!(should_reset_with_setting(true, &current, &old));
        assert!(!should_reset_with_setting(false, &current, &old));
    }

    #[test]
    fn reset_ignores_gamma_and_chao_garden() {
        let gamma_old = GameState {
            curr_character: 6,
            ..GameState::default()
        };
        let chao_garden_current = GameState {
            game_mode: 12,
            level_copy: 32,
            ..GameState::default()
        };
        let menu_current = GameState {
            game_mode: 12,
            ..GameState::default()
        };

        assert!(!should_reset_with_setting(true, &menu_current, &gamma_old));
        assert!(!should_reset_with_setting(
            true,
            &chao_garden_current,
            &GameState::default()
        ));
    }

    #[test]
    fn frame_counted_game_time_removes_loads() {
        let mut run = RunState::new(Instant::now());
        let old = GameState {
            global_frame_count: 100,
            output_frame_count: 50,
            ..GameState::default()
        };
        let current = GameState {
            global_frame_count: 110,
            output_frame_count: 80,
            ..old
        };

        update_game_time(&current, &old, &mut run);

        assert_eq!(run.total_frames, 10);
        assert_eq!(
            game_time(&run),
            Duration::saturating_seconds_f64(10.0 / 60.0)
        );
    }

    #[test]
    fn menu_time_uses_output_frames() {
        let mut run = RunState::new(Instant::now());
        let old = GameState {
            global_frame_count: 100,
            output_frame_count: 50,
            game_mode: 12,
            ..GameState::default()
        };
        let current = GameState {
            global_frame_count: 100,
            output_frame_count: 80,
            ..old
        };

        update_game_time(&current, &old, &mut run);

        assert_eq!(run.total_frames, 30);
    }

    #[test]
    fn cutscene_time_counts_double_frames() {
        let mut run = RunState::new(Instant::now());
        let old = GameState {
            global_frame_count: 100,
            ..GameState::default()
        };
        let current = GameState {
            global_frame_count: 110,
            in_cutscene: 1,
            ..old
        };

        update_game_time(&current, &old, &mut run);

        assert_eq!(run.total_frames, 20);
    }

    /// Every setting off. `Settings` has no `Default` (the `Title` marker lacks one), and
    /// it holds only `bool`s and the unit struct `Title`, so all-zero bytes are valid.
    fn no_settings() -> Settings {
        // SAFETY: Settings contains only bool fields and zero-sized Title markers.
        unsafe { std::mem::zeroed() }
    }

    fn fresh_run() -> RunState {
        RunState::new(Instant::now())
    }

    #[test]
    fn character_split_can_ban_trial_mode() {
        let old = story_select_state();
        let current = GameState {
            game_mode: 9,
            ..old
        };
        let mut settings = no_settings();
        settings.char_split = true;

        assert!(should_split(&settings, &current, &old, &mut fresh_run()));
        settings.char_split_ban_trial = true;
        assert!(!should_split(&settings, &current, &old, &mut fresh_run()));
    }

    #[test]
    fn event_split_needs_enabled_event_and_no_story_select() {
        let old = GameState::default();
        let mut current = GameState::default();
        current.event_bytes[36] = 1;
        let mut settings = no_settings();

        assert!(!should_split(&settings, &current, &old, &mut fresh_run()));
        settings.gamma_sonic = true;
        assert!(should_split(&settings, &current, &old, &mut fresh_run()));

        // Event bytes changing while the story select is open must not split.
        let in_menu = GameState {
            game_status: 21,
            ..current
        };
        assert!(!should_split(&settings, &in_menu, &old, &mut fresh_run()));
        // An enabled event staying unchanged does not split again.
        assert!(!should_split(
            &settings,
            &current,
            &current,
            &mut fresh_run()
        ));
    }

    #[test]
    fn sky_chase_ignore_suppresses_other_splits() {
        let old = GameState::default();
        let mut current = GameState {
            level: 36,
            ..GameState::default()
        };
        current.event_bytes[36] = 1;
        let mut settings = no_settings();
        settings.gamma_sonic = true;

        assert!(should_split(&settings, &current, &old, &mut fresh_run()));
        settings.ignore_sky_chase_1 = true;
        assert!(!should_split(&settings, &current, &old, &mut fresh_run()));
        // Sky Chase 2 is a separate setting.
        let act_two = GameState {
            level: 37,
            ..current
        };
        assert!(should_split(&settings, &act_two, &old, &mut fresh_run()));
        settings.ignore_sky_chase_2 = true;
        assert!(!should_split(&settings, &act_two, &old, &mut fresh_run()));
    }

    #[test]
    fn perfect_chaos_splits_once_on_health_reaching_zero() {
        let old = GameState {
            level: 19,
            boss_health: 2,
            ..GameState::default()
        };
        let current = GameState {
            boss_health: 0,
            ..old
        };
        let mut settings = no_settings();

        assert!(!should_split(&settings, &current, &old, &mut fresh_run()));
        settings.perfect_chaos = true;
        assert!(should_split(&settings, &current, &old, &mut fresh_run()));
        assert!(!should_split(
            &settings,
            &current,
            &current,
            &mut fresh_run()
        ));
    }

    #[test]
    fn chaos_6_uses_the_timer_edge_for_knuckles_and_the_event_for_sonic() {
        let knuckles_fighting = GameState {
            level: 18,
            curr_character: 3,
            game_status: 15,
            timer_start: 1,
            boss_health: 1,
            ..GameState::default()
        };
        let knuckles_defeated_boss = GameState {
            timer_start: 0,
            boss_health: 0,
            ..knuckles_fighting
        };
        let mut settings = no_settings();
        let mut run = fresh_run();

        assert!(!should_split(
            &settings,
            &knuckles_defeated_boss,
            &knuckles_fighting,
            &mut run
        ));
        settings.chaos_6 = true;
        assert!(should_split(
            &settings,
            &knuckles_defeated_boss,
            &knuckles_fighting,
            &mut run
        ));
        assert!(!should_split(
            &settings,
            &knuckles_defeated_boss,
            &knuckles_defeated_boss,
            &mut run
        ));

        let sonic_fighting = GameState {
            curr_character: 0,
            ..knuckles_fighting
        };
        let sonic_defeated_boss = GameState {
            curr_character: 0,
            ..knuckles_defeated_boss
        };
        let mut run = fresh_run();
        settings.six_sonic = true;
        assert!(!should_split(
            &settings,
            &sonic_defeated_boss,
            &sonic_fighting,
            &mut run
        ));

        let mut sonic_event = sonic_defeated_boss;
        sonic_event.event_bytes[50] = 1;
        assert!(should_split(
            &settings,
            &sonic_event,
            &sonic_defeated_boss,
            &mut run
        ));
        assert!(!should_split(
            &settings,
            &sonic_event,
            &sonic_event,
            &mut run
        ));
    }

    #[test]
    fn upgrade_split_needs_increase_and_a_loaded_level() {
        let old = GameState {
            level: 5,
            ..GameState::default()
        };
        let current = GameState {
            lightshoes: 1,
            ..old
        };
        let mut settings = no_settings();
        settings.sonic_powerup1 = true;

        assert!(should_split(&settings, &current, &old, &mut fresh_run()));
        assert!(!should_split(&settings, &old, &current, &mut fresh_run()));
        let at_level_zero = GameState {
            level: 0,
            ..current
        };
        assert!(!should_split(
            &settings,
            &at_level_zero,
            &old,
            &mut fresh_run()
        ));
    }

    #[test]
    fn emblem_in_action_stage_splits_after_delay_and_only_once() {
        let old = GameState {
            level: 5,
            global_frame_count: 1000,
            ..GameState::default()
        };
        let mut current = GameState {
            global_frame_count: 1001,
            ..old
        };
        current.emblem_bytes[0] = 1;
        let mut run = fresh_run();

        assert!(!emblem_split(&current, &old, &mut run));
        assert_eq!(run.delay, 1001 + 190);

        let waiting = GameState {
            global_frame_count: 1100,
            ..current
        };
        assert!(!emblem_split(&waiting, &current, &mut run));
        let due = GameState {
            global_frame_count: 1191,
            ..current
        };
        assert!(emblem_split(&due, &waiting, &mut run));
        assert_eq!(run.delay, 0);
        assert!(!emblem_split(&due, &due, &mut run));
    }

    #[test]
    fn emblem_in_later_levels_splits_immediately_but_not_in_main_menu() {
        let old = GameState {
            level: 20,
            ..GameState::default()
        };
        let mut current = old;
        current.emblem_bytes[3] = 1;

        assert!(emblem_split(&current, &old, &mut fresh_run()));
        let in_menu = GameState {
            game_mode: 12,
            ..current
        };
        assert!(!emblem_split(&in_menu, &old, &mut fresh_run()));
    }

    #[test]
    fn credits_add_character_specific_frames_once() {
        let mut run = fresh_run();
        let old = GameState {
            game_mode: 21,
            curr_character: 2,
            ..GameState::default()
        };
        let current = GameState {
            game_mode: 22,
            ..old
        };
        update_game_time(&current, &old, &mut run);
        assert_eq!(run.credits_counter, 3599);
        assert_eq!(run.total_frames, 0);

        run.credits_counter = 1;
        update_game_time(&current, &current, &mut run);
        assert_eq!(run.total_frames, 14_657);
        // The counter goes negative afterwards, so the bonus is not repeated.
        update_game_time(&current, &current, &mut run);
        assert_eq!(run.total_frames, 14_657);
    }

    #[test]
    fn enter_splits_fire_only_on_their_level_transition() {
        type Case = (fn(&mut Settings), u8, u8);
        let cases: [Case; 17] = [
            (|s| s.enter_egg_hornet = true, 20, 33),
            (|s| s.enter_chaos_2 = true, 16, 26),
            (|s| s.enter_chaos_4 = true, 17, 33),
            (|s| s.enter_chaos_6 = true, 18, 29),
            (|s| s.enter_beta_1 = true, 24, 33),
            (|s| s.enter_beta_2 = true, 25, 29),
            (|s| s.eec = true, 1, 26),
            (|s| s.ewv = true, 2, 33),
            (|s| s.ec = true, 9, 26),
            (|s| s.eic = true, 8, 33),
            (|s| s.etp = true, 3, 26),
            (|s| s.esh = true, 4, 26),
            (|s| s.erm = true, 5, 33),
            (|s| s.esd = true, 6, 29),
            (|s| s.elw = true, 7, 33),
            (|s| s.efe = true, 10, 33),
            (|s| s.ehs = true, 12, 32),
        ];
        for (enable, level, from) in cases {
            let old = GameState {
                level: from as _,
                ..GameState::default()
            };
            let current = GameState {
                level: level as _,
                ..old
            };
            let mut settings = no_settings();
            assert!(
                !should_split(&settings, &current, &old, &mut fresh_run()),
                "{from}->{level} splits with its setting off"
            );
            enable(&mut settings);
            assert!(
                should_split(&settings, &current, &old, &mut fresh_run()),
                "{from}->{level} should split"
            );
            assert!(
                !should_split(&settings, &current, &current, &mut fresh_run()),
                "{level} must not split while staying in the level"
            );
            let wrong_origin = GameState { level: 0, ..old };
            assert!(
                !should_split(&settings, &current, &wrong_origin, &mut fresh_run()),
                "0->{level} must not split"
            );
        }
    }

    #[test]
    fn zero_splits_once_when_health_drops_from_one_to_zero() {
        let old = GameState {
            level: 23,
            boss_health: 1,
            ..GameState::default()
        };
        let current = GameState {
            boss_health: 0,
            ..old
        };
        let mut settings = no_settings();

        assert!(!should_split(&settings, &current, &old, &mut fresh_run()));
        settings.zero = true;
        assert!(should_split(&settings, &current, &old, &mut fresh_run()));
        assert!(!should_split(
            &settings,
            &current,
            &current,
            &mut fresh_run()
        ));
    }

    #[test]
    fn timer_edge_final_bosses_need_their_variant_marker() {
        // Egg Walker (level 21, walker_value 9) and Beta MK2 (level 25, mk2_value 255)
        // split when the boss timer stops with health at zero.
        for (level, walker, mk2) in [(21, 9, 0), (25, 0, 255)] {
            let old = GameState {
                level,
                walker_value: walker,
                mk2_value: mk2,
                timer_start: 1,
                boss_health: 0,
                ..GameState::default()
            };
            let current = GameState {
                timer_start: 0,
                ..old
            };
            let mut settings = no_settings();
            settings.egg_walker = true;
            settings.beta_mk2 = true;
            assert!(should_split(&settings, &current, &old, &mut fresh_run()));
            assert!(!should_split(
                &settings,
                &current,
                &current,
                &mut fresh_run()
            ));
            let alive = GameState {
                boss_health: 3,
                ..current
            };
            assert!(!should_split(&settings, &alive, &old, &mut fresh_run()));
            let other_variant = GameState {
                walker_value: 0,
                mk2_value: 0,
                ..current
            };
            let old_other = GameState {
                walker_value: 0,
                mk2_value: 0,
                ..old
            };
            assert!(!should_split(
                &settings,
                &other_variant,
                &old_other,
                &mut fresh_run()
            ));
        }
    }

    #[test]
    fn e101_splits_on_flag_increase_for_big_only_character_six() {
        let old = GameState {
            curr_character: 6,
            ..GameState::default()
        };
        let current = GameState {
            e101_flag: 1,
            ..old
        };
        let mut settings = no_settings();
        assert!(!should_split(&settings, &current, &old, &mut fresh_run()));
        settings.e101 = true;
        assert!(should_split(&settings, &current, &old, &mut fresh_run()));
        let other = GameState {
            curr_character: 0,
            ..current
        };
        assert!(!should_split(&settings, &other, &old, &mut fresh_run()));
    }

    #[test]
    fn each_event_byte_is_wired_to_its_own_setting() {
        type Case = (fn(&mut Settings), usize);
        let cases: [Case; 14] = [
            (|s| s.gamma_sonic = true, 36),
            (|s| s.knux_sonic = true, 37),
            (|s| s.zero_sonic = true, 48),
            (|s| s.four_sonic = true, 49),
            (|s| s.six_sonic = true, 50),
            (|s| s.egg_hornet_sonic = true, 51),
            (|s| s.gamma_tails = true, 95),
            (|s| s.knux_tails = true, 96),
            (|s| s.four_tails = true, 103),
            (|s| s.egg_hornet_tails = true, 105),
            (|s| s.sonic_knuckles = true, 158),
            (|s| s.two_knuckles = true, 163),
            (|s| s.four_knuckles = true, 165),
            (|s| s.sonic_gamma = true, 282),
        ];
        let old = GameState::default();
        for (enable, index) in cases {
            let mut current = GameState::default();
            current.event_bytes[index] = 1;
            let mut settings = no_settings();
            assert!(
                !should_split(&settings, &current, &old, &mut fresh_run()),
                "event {index} splits with its setting off"
            );
            enable(&mut settings);
            assert!(
                should_split(&settings, &current, &old, &mut fresh_run()),
                "event {index} should split"
            );
            // A neighbouring, unmapped event byte never splits.
            let mut other = GameState::default();
            other.event_bytes[index + 1] = 1;
            assert!(
                !should_split(&settings, &other, &old, &mut fresh_run()),
                "event {} must not split",
                index + 1
            );
        }
    }

    #[test]
    fn act_split_ignores_hubs_and_non_stage_levels() {
        let mut settings = no_settings();
        let old = GameState {
            level: 1,
            act: 0,
            ..GameState::default()
        };
        let current = GameState { act: 1, ..old };
        assert!(!should_split(&settings, &current, &old, &mut fresh_run()));
        settings.act_split = true;
        assert!(should_split(&settings, &current, &old, &mut fresh_run()));
        assert!(!should_split(
            &settings,
            &current,
            &current,
            &mut fresh_run()
        ));
        for level in [0, 15, 22, 26, 29, 32, 33, 37] {
            let old = GameState {
                level,
                act: 0,
                ..GameState::default()
            };
            let current = GameState { act: 1, ..old };
            assert!(
                !should_split(&settings, &current, &old, &mut fresh_run()),
                "act change in level {level} must not split"
            );
        }
    }

    #[test]
    fn character_gated_misc_splits_need_their_character() {
        // Mystic Ruins act 0 -> 2 jungle transitions belong to Amy only.
        let old = GameState {
            level: 33,
            act: 0,
            curr_character: 5,
            ..GameState::default()
        };
        let current = GameState { act: 2, ..old };
        let mut settings = no_settings();
        settings.enter_jungle_to_fe_amy = true;
        assert!(should_split(&settings, &current, &old, &mut fresh_run()));
        let sonic = GameState {
            curr_character: 0,
            ..current
        };
        assert!(!should_split(&settings, &sonic, &old, &mut fresh_run()));

        // Leaving the jungle back to the Egg Carrier route is the reverse edge.
        let back_old = GameState { act: 2, ..old };
        let back = GameState { act: 0, ..old };
        let mut settings = no_settings();
        settings.exit_jungle_to_ec_amy = true;
        assert!(should_split(&settings, &back, &back_old, &mut fresh_run()));
        assert!(!should_split(&settings, &back, &back, &mut fresh_run()));
    }
}
