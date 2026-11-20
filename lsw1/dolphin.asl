state("Dolphin") {}

startup {
    Assembly.Load(File.ReadAllBytes("Components/emu-help-v2")).CreateInstance("GCN");

    settings.Add("newgame", true, "Start on New Game");
    settings.Add("maul", true, "End on Darth Maul Outro (Any%)");
    settings.Add("room", false, "Split on every room (Any%)");
    settings.Add("vader", false, "End on Darth Vader Outro (All Minikits)");
    settings.Add("anh", false, "End on A New Hope Outro (100% NSR)");
    settings.Add("ngplus", false, "NG+: reset on New Game instead of the title screen");
    settings.Add("ruinwarp", false, "Split on Ruin wrong warps (NG+: to the Episode I and II Endings)");

    // Story chapters as level-table index ranges (the table at 0x801D68EC), each from its
    // _Intro to its _Status level. Room splits re-arm when the level moves to another chapter.
    var chapters = new int[][] {
        new[] { 0, 6 }, // Negotiations
        new[] { 7, 13 }, // Invasion
        new[] { 14, 22 }, // Escape
        new[] { 23, 32 }, // Podrace
        new[] { 33, 43 }, // Theed
        new[] { 44, 50 }, // Maul
        new[] { 52, 61 }, // Kamino
        new[] { 62, 71 }, // Factory
        new[] { 72, 75 }, // Jedi Battle
        new[] { 76, 78 }, // Gunship
        new[] { 79, 83 }, // Dooku
        new[] { 86, 87 }, // Dogfight
        new[] { 88, 97 }, // Chancellor
        new[] { 98, 99 }, // Grievous
        new[] { 100, 104 }, // Kashyyyk
        new[] { 105, 110 }, // Ruin
        new[] { 111, 115 }, // Vader
        new[] { 119, 122 }, // Bonus
    };
    vars.ChapterOf = (Func<int, int>)(level => {
        foreach (var c in chapters) if (level >= c[0] && level <= c[1]) return c[0];
        return -1;
    });

    // Room splits (Any%), each taken once per chapter visit: the chapter (its first level),
    // then entering ("in") or leaving ("out") a door in that chapter, or a level loading
    // (for rooms reached by more than one door or wrong warp, or with no usable door).
    // Chancellor rooms 3-7, Ruin's Temple_C, Vader_B/C and Maul_E (door_17 or the wrong warp)
    // are levels; door names repeat between levels, so doors only count in their own chapter.
    vars.Rooms = new string[] {
        "0 in door_to_hangar",
        "7 in door_into_ruins1",
        "7 in door_into_ruins2",
        "7 out door_into_ruins2",
        "7 in door_swamp",
        "14 in RescueA_RescueB",
        "14 in RescueB_RescueC",
        "14 in RescueC_RescueE",
        "33 in retakeA_retakeB",
        "33 in retakeB_retakeD",
        "33 in retakeD_retakeE",
        "33 in retakeE_retakeF",
        "33 in Palace_Hangar",
        "52 in door1in",
        "52 in door10out",
        "52 out door10out",
        "52 in door2corr",
        "52 in door_bossout",
        "62 in door_4",
        "62 in door_3in",
        "62 in door_5in",
        "62 in door_6in",
        "62 in door_8out",
        "76 in GunshipA_mid",
        "76 in GunshipA_B",
        "79 in door_main_in",
        "88 in door_grev",
        "88 level 89",
        "88 level 90",
        "88 level 91",
        "88 level 92",
        "88 level 93",
        "100 in door_to_beach",
        "100 in door_to_swamp",
        "100 in door_to_forest",
        "105 in door_to_inside",
        "105 level 109",
        "105 in door_to_control",
        "111 level 113",
        "111 level 114",
        "111 in door_fight",
        "44 in door_2",
        "44 in door_3",
        "44 level 48",
        "44 in door_8",
    };
    // The podrace lap door: its first two crossings split, the third rolls into the outro.
    vars.PodraceLapSplits = 2;
}

init {
    vars.Taken = new HashSet<int>();
    vars.Laps = 0;
    vars.Helper.Load = (Func<dynamic, bool>)(emu => {
        emu.Make<byte>("status", 0x802E1CB8);
        emu.Make<int>("gamestate", 0x804093B0);
        // The loaded level's level-table index (gamestate is only the requested one).
        emu.Make<int>("level", 0x804093AC);
        // 1 only after New Game on the title menu (cleared when its intro ends); a loaded save
        // or a level quit into the hub leaves it at 0.
        emu.Make<int>("newgame", 0x80409118);
        emu.Make<byte>("cutscene", 0x801BB45D);
        emu.MakeString("door", 16, 0x801DC964);
        return true;
    });
}

update {
    if (vars.ChapterOf(current.level) != vars.ChapterOf(old.level)) {
        vars.Taken.Clear();
        vars.Laps = 0;
    }
}

start {
    if (settings["newgame"] && old.newgame == 0 && current.newgame != 0) return true;
}

split {
    if (old.status == 0 && current.status == 1 && current.door != "door_A" && current.door != "door_B" && !(current.door == "" && (old.door == "door_A" || old.door == "door_B"))) return true;
    if (settings["room"]) {
        if (vars.ChapterOf(current.level) == 23 && old.door != current.door && current.door == "door_B") {
            vars.Laps++;
            if (vars.Laps <= vars.PodraceLapSplits) return true;
        }
        int chapter = vars.ChapterOf(current.level), requested = vars.ChapterOf(current.gamestate);
        string[] rooms = vars.Rooms;
        for (int i = 0; i < rooms.Length; i++) {
            var room = rooms[i].Split(' ');
            int roomChapter = int.Parse(room[0]);
            bool here = roomChapter == chapter || roomChapter == requested;
            bool reached = room[1] == "level"
                ? current.level == int.Parse(room[2]) && old.level != current.level
                : here && old.door != current.door && (room[1] == "in" ? current.door : old.door) == room[2];
            if (reached && !vars.Taken.Contains(i)) {
                vars.Taken.Add(i);
                return true;
            }
        }
    }
    if (settings["maul"] && current.door == "door_8" && current.cutscene == 1) return true;
    if (settings["vader"] && current.door == "door_fight" && current.cutscene == 1) return true;
    if (settings["anh"] && current.door == "door_from_room2" && current.cutscene == 1) return true;
    // Only the NG+ Ruin wrong warps (to the Episode I and II endings) clear door_to_archive straight to nothing.
    if (settings["ruinwarp"] && old.door == "door_to_archive" && current.door == "") return true;
}

reset {
    // NG+ runs pass through the title screen; reset on the next New Game, and start then
    // runs in the same update. LiveSplit only runs reset while the timer is running or paused,
    // so a finished run needs a manual reset (the wasm also restarts it).
    if (settings["ngplus"]) return old.newgame == 0 && current.newgame != 0;
    return current.gamestate == 124;
}

exit {
    timer.IsGameTimePaused = true;
}