mod common;

use common::tmp_root;
use shelf_store::{scan, Platform};
use tempfile::TempDir;

fn write_rom(d: &TempDir, rel: &str, title: &str) {
    let mut rom = vec![0u8; 0x100];
    rom[0xa0..0xa0 + title.len()].copy_from_slice(title.as_bytes());
    std::fs::write(d.path().join("Games").join(rel), rom).expect("write rom");
}

fn write_png(d: &TempDir, rel: &str) {
    let path = d.path().join("Labels").join(rel);
    std::fs::create_dir_all(path.parent().expect("label has a parent")).expect("create dir");
    std::fs::write(path, b"\x89PNG\r\n\x1a\n").expect("write png");
}

#[test]
fn a_png_in_labels_is_paired_to_its_rom_by_stem() {
    let d = tmp_root();
    write_rom(&d, "GBA/Pokemon Emerald.gba", "POKEMON EMER");
    write_png(&d, "GBA/Pokemon Emerald.png");
    write_rom(&d, "GBA/Advance Wars.gba", "ADVANCEWARS");
    let carts = scan(d.path()).unwrap();
    assert_eq!(carts.len(), 2);
    assert_eq!(carts[0].stem, "Advance Wars");
    assert!(carts[0].label.is_none());
    assert!(
        carts[1].label.is_some(),
        "a label in Labels/ was not picked up"
    );
    let mut emerald = carts[1].clone();
    emerald.read_header();
    assert_eq!(emerald.title, "POKEMON EMER");
}

#[test]
fn a_label_pairs_with_its_rom_whatever_the_case_of_its_name() {
    let d = tmp_root();
    write_rom(&d, "GBA/Pokemon Emerald.gba", "POKEMON EMER");
    write_png(&d, "GBA/pokemon emerald.PNG");
    let carts = scan(d.path()).unwrap();
    assert_eq!(
        carts[0].label.as_deref(),
        Some(d.path().join("Labels/GBA/pokemon emerald.PNG").as_path())
    );
}

#[test]
fn a_scan_reads_names_and_leaves_the_rom_headers_for_later() {
    let d = tmp_root();
    write_rom(&d, "GBA/Advance Wars.gba", "ADVANCEWARS");
    let mut cart = scan(d.path()).unwrap().remove(0);
    assert!(cart.title.is_empty());
    cart.read_header();
    assert_eq!(cart.title, "ADVANCEWARS");
}

#[test]
fn scan_ignores_non_gba_files() {
    let d = tmp_root();
    write_rom(&d, "GBA/Real.gba", "REAL");
    std::fs::write(d.path().join("Games/GBA/notes.txt"), "hi").unwrap();
    assert_eq!(scan(d.path()).unwrap().len(), 1);
}

#[test]
fn an_appledouble_sidecar_is_not_shelved_as_a_cart() {
    let d = tmp_root();
    write_rom(&d, "GBA/Metroid Fusion.gba", "METROID");
    write_rom(&d, "GBA/._Metroid Fusion.gba", "METROID");
    let carts = scan(d.path()).unwrap();
    assert_eq!(carts.len(), 1, "an AppleDouble sidecar reached the shelf");
    assert_eq!(carts[0].stem, "Metroid Fusion");
}

#[test]
fn header_title_of_a_truncated_rom_is_none_not_a_panic() {
    let d = tmp_root();
    std::fs::write(d.path().join("Games/GBA/Tiny.gba"), [0u8; 8]).unwrap();
    let mut cart = scan(d.path()).unwrap().remove(0);
    cart.read_header();
    assert!(cart.title.is_empty());
}

#[test]
fn a_header_title_that_is_not_text_is_dropped_rather_than_mangled() {
    let d = tmp_root();
    let mut rom = vec![0u8; 0x100];
    rom[0xa0..0xac].copy_from_slice(&[0xffu8; 12]);
    std::fs::write(d.path().join("Games/GBA/Garbage.gba"), rom).unwrap();
    let mut cart = scan(d.path()).unwrap().remove(0);
    cart.read_header();
    assert!(cart.title.is_empty());
}

#[test]
fn a_root_with_no_games_directory_scans_as_empty() {
    let d = tempfile::tempdir().unwrap();
    assert!(scan(d.path()).unwrap().is_empty());
}

#[test]
fn a_cart_takes_the_platform_of_the_folder_it_is_in() {
    let d = tmp_root();
    write_rom(&d, "GBA/Metroid Fusion.gba", "METROID");
    std::fs::write(d.path().join("Games/GB/Tetris.gb"), vec![0u8; 0x150]).unwrap();
    std::fs::write(d.path().join("Games/GBC/Chromatic.gbc"), vec![0u8; 0x150]).unwrap();

    let carts = scan(d.path()).unwrap();

    let by_stem = |s: &str| carts.iter().find(|c| c.stem == s).unwrap().platform;
    assert_eq!(by_stem("Metroid Fusion"), Platform::Gba);
    assert_eq!(by_stem("Tetris"), Platform::Gb);
    assert_eq!(by_stem("Chromatic"), Platform::Gbc);
}

#[test]
fn a_gba_rom_in_the_game_boy_folder_does_not_appear() {
    let d = tmp_root();
    std::fs::write(d.path().join("Games/GB/Wrong.gba"), vec![0u8; 0x150]).unwrap();
    assert!(scan(d.path()).unwrap().is_empty());
}

#[test]
fn an_unreadable_platform_folder_does_not_take_the_rest_of_the_library_with_it() {
    let d = tmp_root();
    write_rom(&d, "GBA/Metroid Fusion.gba", "METROID");
    std::fs::write(d.path().join("Games/GB/Tetris.gb"), vec![0u8; 0x150]).unwrap();
    std::fs::remove_dir(d.path().join("Games/GBC")).unwrap();
    std::fs::write(d.path().join("Games/GBC"), b"not a directory").unwrap();

    let carts = scan(d.path()).expect("one bad folder must not fail the whole scan");

    assert_eq!(carts.len(), 2, "the readable shelves were lost too");
    assert!(carts.iter().any(|c| c.stem == "Metroid Fusion"));
    assert!(carts.iter().any(|c| c.stem == "Tetris"));
}

#[test]
fn the_same_stem_on_two_platforms_is_two_carts_with_two_labels() {
    let d = tmp_root();
    write_rom(&d, "GBA/Tetris.gba", "TETRIS");
    std::fs::write(d.path().join("Games/GB/Tetris.gb"), vec![0u8; 0x150]).unwrap();
    std::fs::create_dir_all(d.path().join("Labels/GB")).unwrap();
    std::fs::write(d.path().join("Labels/GB/Tetris.png"), b"\x89PNG\r\n\x1a\n").unwrap();

    let carts = scan(d.path()).unwrap();

    assert_eq!(carts.len(), 2);
    let gb = carts.iter().find(|c| c.platform == Platform::Gb).unwrap();
    let gba = carts.iter().find(|c| c.platform == Platform::Gba).unwrap();
    assert!(gb.label.is_some(), "the Game Boy label was not found");
    assert!(
        gba.label.is_none(),
        "the GBA cart borrowed the Game Boy label"
    );
}

fn write_in(dir: &std::path::Path, rel: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("rom has a parent")).expect("create dir");
    std::fs::write(path, vec![0u8; 0x200]).expect("write rom");
}

#[test]
fn nextui_folders_are_picked_by_the_tag_in_brackets() {
    let d = tmp_root();
    let roms = TempDir::new().expect("roms dir");
    write_in(roms.path(), "Game Boy Advance (GBA)/Minish Cap.gba");
    write_in(roms.path(), "Game Boy (GB)/Metroid II.gb");
    write_in(roms.path(), "Game Boy Color (GBC)/Pokemon Crystal.gbc");
    write_in(
        roms.path(),
        "Super Nintendo Entertainment System (SFC)/Super Metroid.7z",
    );
    write_in(
        roms.path(),
        "Super Nintendo Entertainment System (SFC)/Earthbound.smc",
    );
    write_in(roms.path(), "Sega Genesis (MD)/Gunstar Heroes (USA).md");
    write_in(roms.path(), "Sega Genesis (MD)/00) Hacks (Genesis)/Hack.md");
    write_in(roms.path(), "Sega Master System (SMS)/Alex Kidd.sms");
    write_in(
        roms.path(),
        "Nintendo Entertainment System (FC)/Metroid.nes",
    );
    write_in(roms.path(), "Famicom Disk System (FDS)/Zelda.fds");
    write_in(roms.path(), "GBA Hacks/Hack.gba");
    let carts = shelf_store::scan_in(d.path(), Some(roms.path())).unwrap();
    let got: Vec<(Platform, &str)> = carts
        .iter()
        .map(|c| (c.platform, c.stem.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            (Platform::Gba, "Minish Cap"),
            (Platform::Gb, "Metroid II"),
            (Platform::Gbc, "Pokemon Crystal"),
            (Platform::Nes, "Metroid"),
            (Platform::Sms, "Alex Kidd"),
            (Platform::Md, "Gunstar Heroes (USA)"),
            (Platform::Snes, "Earthbound"),
            (Platform::Snes, "Super Metroid"),
            (Platform::other("FDS", "Famicom Disk System").unwrap(), "Zelda"),
        ]
    );
    assert!(carts[0]
        .rom
        .starts_with(roms.path().join("Game Boy Advance (GBA)")));
}

#[test]
fn with_a_roms_folder_games_under_the_root_are_not_read() {
    let d = tmp_root();
    write_rom(&d, "GBA/Advance Wars.gba", "ADVANCEWARS");
    let roms = TempDir::new().expect("roms dir");
    write_in(roms.path(), "Game Boy Advance (GBA)/Minish Cap.gba");
    let carts = shelf_store::scan_in(d.path(), Some(roms.path())).unwrap();
    assert_eq!(carts.len(), 1);
    assert_eq!(carts[0].stem, "Minish Cap");
}

#[test]
fn a_folder_tag_is_the_last_bracketed_word() {
    assert_eq!(
        shelf_store::folder_tag("Game Boy Advance (GBA)"),
        Some("GBA")
    );
    assert_eq!(shelf_store::folder_tag("Hacks (2) (MGBA)"), Some("MGBA"));
    assert_eq!(shelf_store::folder_tag("Game Boy"), None);
    assert_eq!(shelf_store::tag_platform("mgba"), Some(Platform::Gba));
    assert_eq!(shelf_store::tag_platform("FC"), Some(Platform::Nes));
    assert_eq!(shelf_store::tag_platform("FDS"), None);
    assert_eq!(shelf_store::tag_platform("MD"), Some(Platform::Md));
    assert_eq!(shelf_store::tag_platform("SFC"), Some(Platform::Snes));
    assert_eq!(shelf_store::tag_platform("SNES"), None);
    assert_eq!(shelf_store::tag_platform("SMS"), Some(Platform::Sms));
    assert_eq!(shelf_store::tag_platform("GG"), Some(Platform::Gg));
    assert_eq!(shelf_store::tag_platform("PCE"), Some(Platform::Pce));
    assert_eq!(shelf_store::tag_platform("N64"), Some(Platform::N64));
    assert_eq!(shelf_store::tag_platform("NGPC"), Some(Platform::Ngpc));
    assert_eq!(shelf_store::tag_platform("NGP"), Some(Platform::Ngp));
    assert_eq!(shelf_store::tag_platform("PCECD"), Some(Platform::PceCd));
    assert_eq!(shelf_store::tag_platform("PS"), Some(Platform::Ps));
    assert_eq!(shelf_store::tag_platform("SEGACD"), Some(Platform::SegaCd));
}

#[test]
fn n64_pce_ngp_and_ngpc_folders_are_shelved_and_turbografx_cd_has_its_own_shelf() {
    let roms = TempDir::new().expect("roms dir");
    write_in(roms.path(), "Nintendo 64 (N64)/Wave Race 64.z64");
    write_in(roms.path(), "TurboGrafx-16 (PCE)/Bonk's Adventure.pce");
    write_in(roms.path(), "TurboGrafx-CD (PCE)/Lords of Thunder (USA).chd");
    write_in(roms.path(), "TurboGrafx-CD (PCE)/Ys Book I & II.cue");
    write_in(roms.path(), "TurboGrafx-CD (PCE)/Track 01.bin");
    write_in(roms.path(), "Neo Geo Pocket Color (NGPC)/Sonic Pocket Adventure.ngc");
    write_in(roms.path(), "Neo Geo Pocket (NGP)/Pac-Man.ngp");
    let d = tmp_root();
    let carts = shelf_store::scan_in(d.path(), Some(roms.path())).unwrap();
    let mut got: Vec<(Platform, &str)> = carts.iter().map(|c| (c.platform, c.stem.as_str())).collect();
    got.sort_by_key(|(p, s)| (p.order(), s.to_string()));
    assert_eq!(
        got,
        vec![
            (Platform::N64, "Wave Race 64"),
            (Platform::Pce, "Bonk's Adventure"),
            (Platform::PceCd, "Lords of Thunder (USA)"),
            (Platform::PceCd, "Ys Book I & II"),
            (Platform::Ngp, "Pac-Man"),
            (Platform::Ngpc, "Sonic Pocket Adventure"),
        ]
    );
    assert_eq!(shelf_store::system_platform("TurboGrafx-CD (PCE)"), Some(Platform::PceCd));
    assert_eq!(shelf_store::system_platform("TurboGrafx-16 (PCE)"), Some(Platform::Pce));
}

#[test]
fn without_a_roms_folder_only_platforms_shelf_can_play_are_scanned() {
    let d = tmp_root();
    write_rom(&d, "GBA/Advance Wars.gba", "ADVANCEWARS");
    write_in(&d.path().join("Games"), "MD/Gunstar Heroes.md");
    write_in(&d.path().join("Games"), "SFC/Super Metroid.sfc");
    let carts = shelf_store::scan(d.path()).unwrap();
    let got: Vec<&str> = carts.iter().map(|c| c.stem.as_str()).collect();
    assert_eq!(got, vec!["Advance Wars"]);
}

#[test]
fn reading_a_mega_drive_or_snes_header_leaves_the_title_alone() {
    let roms = TempDir::new().expect("roms dir");
    write_in(roms.path(), "Sega Genesis (MD)/Ristar.md");
    let carts = shelf_store::scan_in(tmp_root().path(), Some(roms.path())).unwrap();
    let mut cart = carts[0].clone();
    cart.read_header();
    assert_eq!(cart.title, "");
}

#[test]
fn zipped_games_are_shelved_from_a_roms_folder_but_not_from_games() {
    let d = tmp_root();
    write_in(&d.path().join("Games"), "GB/Tetris.zip");
    assert!(shelf_store::scan(d.path()).unwrap().is_empty());
    let roms = TempDir::new().expect("roms dir");
    write_in(roms.path(), "Game Boy (GB)/Tetris.zip");
    write_in(roms.path(), "Game Boy Advance (GBA)/Mother 3.7z");
    let mut carts = shelf_store::scan_in(d.path(), Some(roms.path())).unwrap();
    let got: Vec<(Platform, &str)> = carts
        .iter()
        .map(|c| (c.platform, c.stem.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![(Platform::Gba, "Mother 3"), (Platform::Gb, "Tetris")]
    );
    for cart in &mut carts {
        cart.read_header();
        assert_eq!(
            cart.title,
            "",
            "read a header out of {}",
            cart.rom.display()
        );
    }
}

#[test]
fn collection_lists_name_roms_from_the_root_of_the_card() {
    let card = TempDir::new().expect("card");
    let roms = card.path().join("Roms");
    let lists = card.path().join("Collections").join("Top 100");
    std::fs::create_dir_all(&lists).unwrap();
    std::fs::write(
        lists.join("T100 Game Boy.txt"),
        "/Roms/Game Boy (GB)/Tetris.zip\n/Roms/Game Boy (GB)/Dr. Mario.zip\n\n",
    )
    .unwrap();
    std::fs::write(
        lists.join("T100 Sega Genesis.txt"),
        "/Roms/Sega Genesis (MD)/Ristar.md\r\n",
    )
    .unwrap();
    std::fs::write(lists.join("notes.md"), "/Roms/Game Boy (GB)/Kirby.zip\n").unwrap();
    let listed = shelf_store::listed(&lists, &roms);
    assert_eq!(listed.len(), 3);
    assert!(listed.contains(&roms.join("Game Boy (GB)").join("Tetris.zip")));
    assert!(listed.contains(&roms.join("Sega Genesis (MD)").join("Ristar.md")));
    assert!(!listed.contains(&roms.join("Game Boy (GB)").join("Kirby.zip")));
    let one = shelf_store::listed(&lists.join("T100 Sega Genesis.txt"), &roms);
    assert_eq!(one.len(), 1);
}

#[test]
fn listed_games_are_shelved_from_subfolders_too_and_nothing_else_is() {
    let card = TempDir::new().expect("card");
    let roms = card.path().join("Roms");
    write_in(&roms, "Game Boy (GB)/Tetris.zip");
    write_in(&roms, "Game Boy (GB)/Kirby.zip");
    write_in(
        &roms,
        "Game Boy Advance (GBA)/00) Hacks (GameBoy Advance)/Unbound.zip",
    );
    write_in(&roms, "Sega Genesis (MD)/Ristar.md");
    write_in(&roms, "Sega Genesis (MD)/.media/Ristar.png");
    let lists = card.path().join("Top 100");
    std::fs::create_dir_all(&lists).unwrap();
    std::fs::write(
        lists.join("T100.txt"),
        "/Roms/Game Boy (GB)/Tetris.zip\n\
         /Roms/Game Boy Advance (GBA)/00) Hacks (GameBoy Advance)/Unbound.zip\n\
         /Roms/Sega Genesis (MD)/Ristar.md\n\
         /Roms/Sega Genesis (MD)/Not On The Card.md\n\
         /Roms/Sega Master System (SMS)/Alex Kidd.sms\n",
    )
    .unwrap();
    let carts = shelf_store::scan_listed(tmp_root().path(), &roms, &lists);
    let got: Vec<(Platform, &str)> = carts
        .iter()
        .map(|c| (c.platform, c.stem.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            (Platform::Gba, "Unbound"),
            (Platform::Gb, "Tetris"),
            (Platform::Md, "Ristar"),
        ]
    );
    assert_eq!(
        carts[2].label.as_deref(),
        Some(roms.join("Sega Genesis (MD)/.media/Ristar.png").as_path()),
        "a listed game did not take its NextUI box art"
    );
}

#[test]
fn nextui_box_art_in_media_stands_in_for_a_missing_label() {
    let d = tmp_root();
    let roms = TempDir::new().expect("roms dir");
    write_in(roms.path(), "Game Boy Advance (GBA)/Mother 3.zip");
    write_in(roms.path(), "Game Boy Advance (GBA)/Minish Cap.gba");
    write_in(roms.path(), "Game Boy Advance (GBA)/.media/mother 3.png");
    write_in(roms.path(), "Game Boy Advance (GBA)/.media/Minish Cap.png");
    write_png(&d, "GBA/Minish Cap.png");
    let carts = shelf_store::scan_in(d.path(), Some(roms.path())).unwrap();
    let label = |stem: &str| {
        carts
            .iter()
            .find(|c| c.stem == stem)
            .and_then(|c| c.label.clone())
            .expect("no label")
    };
    assert_eq!(
        label("Mother 3"),
        roms.path()
            .join("Game Boy Advance (GBA)/.media/mother 3.png")
    );
    assert_eq!(
        label("Minish Cap"),
        d.path().join("Labels/GBA/Minish Cap.png"),
        "Shelf's own label should win over NextUI's box art"
    );
}

#[test]
fn zipped_games_are_only_read_from_a_roms_folder() {
    let d = tmp_root();
    std::fs::write(d.path().join("Games/GBA/Zipped.zip"), b"PK").expect("write zip");
    assert!(scan(d.path()).unwrap().is_empty());
    let roms = TempDir::new().expect("roms dir");
    write_in(roms.path(), "Game Boy (GB)/Tetris.7z");
    let carts = shelf_store::scan_in(d.path(), Some(roms.path())).unwrap();
    assert_eq!(carts.len(), 1);
    let mut tetris = carts[0].clone();
    tetris.read_header();
    assert_eq!(tetris.title, "", "a header was read out of an archive");
}

#[test]
fn satellaview_games_stay_off_the_snes_shelf() {
    let card = TempDir::new().expect("card");
    let roms = card.path().join("Roms");
    write_in(
        &roms,
        "Super Nintendo Entertainment System (SFC)/F-Zero.sfc",
    );
    write_in(&roms, "Satellaview (SFC)/BS F-Zero Grand Prix.bs.7z");
    let lists = card.path().join("Top 100");
    std::fs::create_dir_all(&lists).unwrap();
    std::fs::write(
        lists.join("T100.txt"),
        "/Roms/Super Nintendo Entertainment System (SFC)/F-Zero.sfc\n\
         /Roms/Satellaview (SFC)/BS F-Zero Grand Prix.bs.7z\n",
    )
    .unwrap();
    let stems = |carts: Vec<shelf_store::Cart>| -> Vec<String> {
        carts.into_iter().map(|c| c.stem).collect()
    };
    assert_eq!(
        stems(shelf_store::scan_listed(tmp_root().path(), &roms, &lists)),
        vec!["F-Zero"]
    );
    assert_eq!(
        stems(shelf_store::scan_in(tmp_root().path(), Some(&roms)).unwrap()),
        vec!["F-Zero"]
    );
    assert_eq!(shelf_store::system_platform("Satellaview (SFC)"), None);
}

#[test]
fn a_game_named_in_several_lists_is_listed_and_shelved_once_in_first_seen_order() {
    let card = TempDir::new().expect("card");
    let roms = card.path().join("Roms");
    write_in(&roms, "Game Boy (GB)/Tetris.gb");
    write_in(&roms, "Game Boy (GB)/Kirby.gb");
    std::fs::create_dir_all(roms.join("Game Boy (GB)").join("Folder.gb")).unwrap();
    let lists = card.path().join("Lists");
    std::fs::create_dir_all(&lists).unwrap();
    let body = "/Roms/Game Boy (GB)/Tetris.gb\n/Roms/Game Boy (GB)/Folder.gb\n\
                /Roms/Game Boy (GB)/Kirby.gb\n/Roms/Game Boy (GB)/Tetris.gb\n";
    std::fs::write(lists.join("a.txt"), body).unwrap();
    std::fs::write(lists.join("b.txt"), body).unwrap();
    let listed = shelf_store::listed(&lists, &roms);
    assert_eq!(listed.len(), 3, "{listed:?}");
    assert_eq!(listed[0], roms.join("Game Boy (GB)").join("Tetris.gb"));
    let carts = shelf_store::scan_listed(tmp_root().path(), &roms, &lists);
    let stems: Vec<&str> = carts.iter().map(|c| c.stem.as_str()).collect();
    assert_eq!(stems, ["Kirby", "Tetris"], "a folder is not a cart");
}

#[test]
fn a_roms_folder_is_read_once_for_every_platform_and_each_gets_its_own_folders() {
    let card = TempDir::new().expect("card");
    let roms = card.path().join("Roms");
    write_in(&roms, "Game Boy (GB)/B.gb");
    write_in(&roms, "00) Hacks (GB)/A.gb");
    write_in(&roms, "Game Boy Color (GBC)/C.gbc");
    write_in(&roms, "Super Nintendo (SFC)/D.sfc");
    write_in(&roms, ".Hidden (GB)/E.gb");
    std::fs::write(roms.join("Not A Folder (GB)"), b"x").unwrap();
    let carts = shelf_store::scan_in(tmp_root().path(), Some(&roms)).unwrap();
    let got: Vec<(Platform, &str)> = carts
        .iter()
        .map(|c| (c.platform, c.stem.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            (Platform::Gb, "A"),
            (Platform::Gb, "B"),
            (Platform::Gbc, "C"),
            (Platform::Snes, "D"),
        ]
    );
}

#[test]
fn a_console_no_list_names_shows_all_its_games() {
    let d = tmp_root();
    let card = TempDir::new().expect("card");
    let roms = card.path().join("Roms");
    write_in(&roms, "Game Boy (GB)/Tetris.gb");
    write_in(&roms, "Game Boy (GB)/Kirby.gb");
    write_in(&roms, "Sega Game Gear (GG)/Shinobi.gg");
    let lists = card.path().join("lists");
    std::fs::create_dir_all(&lists).unwrap();
    std::fs::write(lists.join("T100 Game Boy.txt"), "/Roms/Game Boy (GB)/Tetris.gb\n").unwrap();
    let carts = shelf_store::scan_listed(d.path(), &roms, &lists);
    let got: Vec<&str> = carts.iter().map(|c| c.stem.as_str()).collect();
    assert_eq!(got, ["Tetris", "Shinobi"], "Kirby is not listed; Game Gear has no list");
}

#[test]
fn sega_cd_and_playstation_are_shelved_with_each_game_once() {
    let roms = TempDir::new().expect("roms dir");
    write_in(roms.path(), "Sega CD (SEGACD)/Final Fight CD.chd");
    write_in(roms.path(), "Sega CD (SEGACD)/MegaRace.iso");
    write_in(roms.path(), "Sega CD (SEGACD)/Night Trap (Disc 1).cue");
    write_in(roms.path(), "Sega CD (SEGACD)/Night Trap (Disc 2).cue");
    write_in(roms.path(), "Sega CD (SEGACD)/Night Trap (USA) (Disc 1).bin");
    write_in(roms.path(), "Sony PlayStation (PS)/Ace Combat 2.chd");
    write_in(roms.path(), "Sony PlayStation (PS)/Armored Core - Master of Arena.m3u");
    write_in(roms.path(), "Sony PlayStation (PS)/.hidden/Armored Core - Master of Arena (Disc 1).chd");
    write_in(roms.path(), "Sony PlayStation (PS)/.hidden/Armored Core - Master of Arena (Disc 2).chd");
    let d = tmp_root();
    let carts = shelf_store::scan_in(d.path(), Some(roms.path())).unwrap();
    let mut got: Vec<(Platform, &str)> = carts.iter().map(|c| (c.platform, c.stem.as_str())).collect();
    got.sort_by_key(|(p, s)| (p.order(), s.to_string()));
    assert_eq!(
        got,
        vec![
            (Platform::SegaCd, "Final Fight CD"),
            (Platform::SegaCd, "MegaRace"),
            (Platform::SegaCd, "Night Trap (Disc 1)"),
            (Platform::Ps, "Ace Combat 2"),
            (Platform::Ps, "Armored Core - Master of Arena"),
        ]
    );
}

#[test]
fn a_playlist_hides_the_discs_it_lists_and_ignores_comments() {
    let roms = TempDir::new().expect("roms dir");
    let dir = roms.path().join("Sony PlayStation (PS)");
    write_in(roms.path(), "Sony PlayStation (PS)/Dino (Disc 1).chd");
    write_in(roms.path(), "Sony PlayStation (PS)/Dino (Disc 2).chd");
    write_in(roms.path(), "Sony PlayStation (PS)/Twin.chd");
    write_in(roms.path(), "Sony PlayStation (PS)/Lonely (Disc 2).chd");
    std::fs::write(
        dir.join("Dino.m3u"),
        "# discs\n\nDino (Disc 1).chd\r\nDino (Disc 2).chd\n",
    )
    .unwrap();
    let d = tmp_root();
    let carts = shelf_store::scan_in(d.path(), Some(roms.path())).unwrap();
    let mut stems: Vec<&str> = carts.iter().map(|c| c.stem.as_str()).collect();
    stems.sort();
    assert_eq!(stems, vec!["Dino", "Lonely (Disc 2)", "Twin"]);
}
