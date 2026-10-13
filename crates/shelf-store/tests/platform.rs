use shelf_store::Platform;

#[test]
fn every_platform_has_a_directory() {
    let names: Vec<&str> = Platform::ALL.iter().map(|p| p.dir_name()).collect();
    assert_eq!(names, vec!["GBA", "GB", "GBC", "GG", "FC", "SMS", "MD", "SFC", "N64", "PCE", "PCECD", "SEGACD", "PS", "NGP", "NGPC"]);
}

#[test]
fn every_platform_is_a_shelf_of_its_own() {
    assert_eq!(
        Platform::ALL,
        [
            Platform::Gba,
            Platform::Gb,
            Platform::Gbc,
            Platform::Gg,
            Platform::Nes,
            Platform::Sms,
            Platform::Md,
            Platform::Snes,
            Platform::N64,
            Platform::Pce,
            Platform::PceCd,
            Platform::SegaCd,
            Platform::Ps,
            Platform::Ngp,
            Platform::Ngpc
        ]
    );
    for (i, p) in Platform::ALL.iter().enumerate() {
        assert!(
            !Platform::ALL[..i].contains(p),
            "{p:?} is named twice in the ring, so two shelves would hold one platform"
        );
    }
}

#[test]
fn each_platform_takes_only_its_own_extensions() {
    assert!(Platform::Gba.accepts(std::path::Path::new("Metroid Fusion.gba")));
    assert!(!Platform::Gba.accepts(std::path::Path::new("Tetris.gb")));
    assert!(Platform::Gb.accepts(std::path::Path::new("Tetris.gb")));
    assert!(Platform::Gb.accepts(std::path::Path::new("Tetris.gbc")));
    assert!(!Platform::Gb.accepts(std::path::Path::new("Metroid Fusion.gba")));
    assert!(Platform::Gba.accepts(std::path::Path::new("Shrek.GBA")));
}

#[test]
fn nes_mega_drive_and_snes_take_their_own_roms() {
    for name in ["Ristar.md", "Ristar.gen", "Ristar.smd", "Ristar.bin"] {
        assert!(Platform::Md.accepts(std::path::Path::new(name)), "{name}");
    }
    for name in ["Earthbound.smc", "Earthbound.sfc", "Earthbound.bs"] {
        assert!(Platform::Snes.accepts(std::path::Path::new(name)), "{name}");
    }
    for name in ["Metroid.nes", "Metroid.unf"] {
        assert!(Platform::Nes.accepts(std::path::Path::new(name)), "{name}");
    }
    assert!(!Platform::Nes.accepts(std::path::Path::new("Ristar.md")));
    assert!(!Platform::Md.accepts(std::path::Path::new("Earthbound.smc")));
    assert!(!Platform::Snes.accepts(std::path::Path::new("Ristar.md")));
}

#[test]
fn only_the_game_boy_family_has_a_games_folder_on_the_content_root() {
    let folders: Vec<Platform> = Platform::ALL
        .into_iter()
        .filter(|p| p.has_games_folder())
        .collect();
    assert_eq!(folders, vec![Platform::Gba, Platform::Gb, Platform::Gbc]);
}

#[test]
fn zip_and_7z_are_archives_and_roms_are_not() {
    assert!(shelf_store::is_archive(std::path::Path::new("Tetris.zip")));
    assert!(shelf_store::is_archive(std::path::Path::new(
        "Chrono Trigger.7Z"
    )));
    assert!(!shelf_store::is_archive(std::path::Path::new("Tetris.gb")));
    assert!(!Platform::Gb.accepts(std::path::Path::new("Tetris.zip")));
}

#[test]
fn emulator_choices_start_with_nextuis_usual_pak() {
    use shelf_store::{emus_for, Platform};
    let tags = |p| emus_for(p).iter().map(|e| e.tag).collect::<Vec<_>>();
    assert_eq!(tags(Platform::Gba), ["GBA", "MGBA"]);
    assert_eq!(tags(Platform::Gb), ["GB", "SGB"]);
    assert_eq!(tags(Platform::Snes), ["SFC", "SUPA"]);
    assert!(emus_for(Platform::Md).is_empty(), "Mega Drive has one pak");
}

#[test]
fn a_games_emulator_choice_round_trips_and_steps_through_the_choices() {
    use shelf_store::{folder_pak, next_tag, read_emus, write_emus};
    use std::path::{Path, PathBuf};
    let d = tempfile::tempdir().unwrap();
    let rom = PathBuf::from("/Roms/Game Boy Advance (GBA)/Emerald.gba");
    let mut emus = std::collections::HashMap::new();
    emus.insert(rom.clone(), "MGBA".to_string());
    write_emus(d.path(), &emus).unwrap();
    assert_eq!(read_emus(d.path()), emus);
    assert_eq!(next_tag(&["GBA", "MGBA"], "GBA", true), Some("MGBA"));
    assert_eq!(next_tag(&["GBA", "MGBA"], "MGBA", true), Some("GBA"));
    assert_eq!(next_tag(&["GBA", "MGBA"], "GBA", false), Some("MGBA"));
    assert_eq!(next_tag(&[], "GBA", true), None);
    assert_eq!(
        folder_pak(Path::new("/Roms"), Path::new("/Roms/Game Boy (GB)/00) Best (Gameboy)/Wario.gb")),
        Some("GB".to_string()),
        "a game in a subfolder takes its system folder's tag"
    );
}
