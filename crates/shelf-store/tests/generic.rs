//! Systems with no cart of their own are shelved from NextUI's Roms
//! folders. One test, since the emulator folders come from the environment.

use shelf_store::Platform;
use tempfile::TempDir;

fn write_in(dir: &std::path::Path, rel: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"x").unwrap();
}

#[test]
fn tagged_folders_with_games_and_a_pak_get_a_shelf() {
    let card = TempDir::new().unwrap();
    let roms = card.path().join("Roms");
    write_in(&roms, "Atari Lynx (LYNX)/California Games.lnx");
    write_in(&roms, "Atari Lynx (LYNX)/Gauntlet.lnx");
    write_in(&roms, "Atari Lynx (LYNX)/notes.txt");
    write_in(&roms, "Commodore 64 (C64)/Disc.cue");
    write_in(&roms, "Commodore 64 (C64)/Disc.bin");
    write_in(&roms, "Commodore 64 (C64)/Other.d64");
    write_in(&roms, "_Collections/List.txt");
    write_in(&roms, "CPS3/sfiii.zip");
    write_in(&roms, "Satellaview (SFC)/BS Game.bs");
    std::fs::create_dir_all(roms.join("Atari 5200 (A5200)")).unwrap();
    let root = card.path().join("Shelf");

    let stems = |p: Platform, carts: &[shelf_store::Cart]| -> Vec<String> {
        carts
            .iter()
            .filter(|c| c.platform == p)
            .map(|c| c.stem.clone())
            .collect()
    };

    // No emulator folders known: every tagged folder with a game counts.
    std::env::remove_var("SHELF_EMU_DIRS");
    let carts = shelf_store::scan_in(&root, Some(&roms)).unwrap();
    let lynx = Platform::other("LYNX", "x").unwrap();
    let c64 = Platform::other("C64", "x").unwrap();
    assert_eq!(lynx.name(), "Atari Lynx");
    assert_eq!(c64.name(), "Commodore 64");
    assert_eq!(stems(lynx, &carts), vec!["California Games", "Gauntlet"]);
    assert_eq!(stems(c64, &carts), vec!["Disc", "Other"]);
    assert!(carts.iter().all(|c| !c.platform.is_other() || [lynx, c64].contains(&c.platform)));
    assert!(!carts.iter().any(|c| c.stem == "BS Game" || c.stem == "sfiii"));
    assert!(!Platform::all().iter().any(|p| p.dir_name() == "A5200"));

    // With emulator folders known, a console whose pak is missing stays off.
    let emus = card.path().join("Emus");
    write_in(&emus, "LYNX.pak/launch.sh");
    std::env::set_var("SHELF_EMU_DIRS", &emus);
    let carts = shelf_store::scan_in(&root, Some(&roms)).unwrap();
    assert_eq!(stems(lynx, &carts).len(), 2);
    assert!(stems(c64, &carts).is_empty());
    std::env::remove_var("SHELF_EMU_DIRS");
}
