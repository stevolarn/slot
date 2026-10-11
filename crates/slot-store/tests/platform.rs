use slot_store::Platform;

#[test]
fn every_platform_has_a_directory() {
    let names: Vec<&str> = Platform::ALL.iter().map(|p| p.dir_name()).collect();
    assert_eq!(names, vec!["GBA", "GB", "GBC", "FC", "MD", "SFC"]);
}

#[test]
fn every_platform_is_a_shelf_of_its_own() {
    assert_eq!(
        Platform::ALL,
        [
            Platform::Gba,
            Platform::Gb,
            Platform::Gbc,
            Platform::Nes,
            Platform::Md,
            Platform::Snes
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
fn mega_drive_and_snes_take_their_roms_and_archives() {
    for name in [
        "Ristar.md",
        "Ristar.gen",
        "Ristar.smd",
        "Ristar.bin",
        "Ristar.zip",
    ] {
        assert!(Platform::Md.accepts(std::path::Path::new(name)), "{name}");
    }
    for name in [
        "Earthbound.smc",
        "Earthbound.sfc",
        "Earthbound.7z",
        "Earthbound.zip",
    ] {
        assert!(Platform::Snes.accepts(std::path::Path::new(name)), "{name}");
    }
    for name in ["Metroid.nes", "Metroid.unf", "Metroid.zip", "Metroid.7z"] {
        assert!(Platform::Nes.accepts(std::path::Path::new(name)), "{name}");
    }
    assert!(!Platform::Nes.accepts(std::path::Path::new("Ristar.md")));
    assert!(!Platform::Md.accepts(std::path::Path::new("Earthbound.smc")));
    assert!(!Platform::Snes.accepts(std::path::Path::new("Ristar.md")));
}

#[test]
fn only_the_game_boy_family_runs_on_slots_own_cores() {
    let built_in: Vec<Platform> = Platform::ALL.into_iter().filter(|p| p.built_in()).collect();
    assert_eq!(built_in, vec![Platform::Gba, Platform::Gb, Platform::Gbc]);
}
