mod common;

use common::{boot, insert, tmp_root_with_carts};
use shelf::app::{App, Phase};
use shelf_input::{Action, Btn};
use shelf_store::{read_slot_state, write_slot_state, Platform, SlotState};

fn remembered(cart: &str) -> SlotState {
    SlotState {
        cart: Some(cart.into()),
        clock_set: true,
        utc_offset_min: 0,
        ..Default::default()
    }
}

fn settle(a: &mut App) {
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
}

#[test]
fn boot_with_a_remembered_cart_shows_the_shelf_resting_on_it() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald", "Fire Emblem"]);
    write_slot_state(d.path(), &remembered("Emerald")).unwrap();
    let a = App::boot(d.path());
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "boot must show the shelf, not put the cart in"
    );
    assert_eq!(a.selected_stem(), Some("Emerald"));
}

#[test]
fn boot_with_an_empty_slot_shows_the_shelf() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let a = boot(d.path());
    assert!(matches!(a.phase(), Phase::Shelf));
}

#[test]
fn boot_with_a_cart_that_no_longer_exists_falls_back_to_the_shelf() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(d.path(), &remembered("Deleted")).unwrap();
    let a = App::boot(d.path());
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(a.selected_stem(), Some("Emerald"));
}

#[test]
fn a_cart_put_in_is_recorded_once_it_is_seated_so_the_shelf_comes_back_to_it() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    insert(&mut a);
    assert_eq!(
        read_slot_state(d.path()).cart,
        None,
        "a cart that has not seated yet is not remembered"
    );
    settle(&mut a);
    assert_eq!(read_slot_state(d.path()).cart, Some("Emerald".into()));
}

#[test]
fn a_cart_that_is_gone_takes_its_shelf_out_of_the_slot_with_it() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            cart: Some("Deleted".into()),
            cart_platform: Some(Platform::Gbc),
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut a = App::boot(d.path());
    a.apply(Action::MuteToggle);
    a.flush_state();
    let s = read_slot_state(d.path());
    assert_eq!(s.cart, None);
    assert_eq!(
        s.cart_platform, None,
        "the card names a shelf beside a line that names no cart"
    );
}

#[test]
fn putting_a_cart_in_preserves_the_levels_already_in_the_file() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            cart: None,
            brightness: 2,
            volume: 35,
            muted: false,
            clock_set: true,
            utc_offset_min: 0,
            ..SlotState::default()
        },
    )
    .unwrap();
    let mut a = App::boot(d.path());
    insert(&mut a);
    settle(&mut a);
    let s = read_slot_state(d.path());
    assert_eq!((s.brightness, s.volume), (2, 35));
    assert_eq!(s.cart, Some("Emerald".into()));
}

#[test]
fn a_loose_card_shows_an_empty_shelf_and_nothing_on_it_is_moved() {
    let d = tempfile::tempdir().unwrap();
    for sub in ["Games", "Labels"] {
        std::fs::create_dir_all(d.path().join(sub)).unwrap();
    }
    std::fs::write(d.path().join("Games/Emerald.gba"), vec![0u8; 0x100]).unwrap();
    std::fs::write(d.path().join("Labels/Emerald.png"), b"png").unwrap();

    let a = App::boot(d.path());

    assert_eq!(
        a.carts().count(),
        0,
        "a loose rom reached the shelf, so something is still reading outside Games/<platform>/"
    );
    assert_eq!(
        std::fs::read(d.path().join("Games/Emerald.gba"))
            .unwrap()
            .len(),
        0x100,
        "the loose rom was moved or disturbed"
    );
    assert_eq!(
        std::fs::read(d.path().join("Labels/Emerald.png")).unwrap(),
        b"png",
        "the loose label was moved"
    );
}

#[test]
fn boot_creates_only_the_folders_the_picker_keeps_things_in() {
    let d = tempfile::tempdir().unwrap();

    App::boot(d.path());

    for name in shelf::root::DIRS {
        assert!(
            d.path().join(name).is_dir(),
            "{name} is missing, so nothing on the card says where its files go"
        );
    }
    let mut got: Vec<String> = std::fs::read_dir(d.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    got.sort();
    let mut want: Vec<String> = shelf::root::DIRS.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(
        got, want,
        "boot made folders for saves, states or BIOS files that NextUI keeps itself"
    );
}

fn two_tetrises() -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Tetris", "Emerald"]);
    common::write_gb_cart(&d, "Tetris", "TETRIS");
    d
}

fn resumed(d: &tempfile::TempDir, platform: Option<Platform>) -> App {
    write_slot_state(
        d.path(),
        &SlotState {
            cart: Some("Tetris".into()),
            cart_platform: platform,
            clock_set: true,
            utc_offset_min: 0,
            ..Default::default()
        },
    )
    .unwrap();
    App::boot(d.path())
}

#[test]
fn the_platform_on_the_card_decides_which_tetris_the_shelf_rests_on() {
    let d = two_tetrises();
    for platform in [Platform::Gb, Platform::Gba] {
        let a = resumed(&d, Some(platform));
        let rom = a.browsed_rom().expect("the shelf rests on nothing");
        assert!(
            rom.ends_with(format!(
                "{}/Tetris.{}",
                platform.dir_name(),
                platform.extensions()[0]
            )),
            "the card named {platform:?} and the shelf rests on {rom:?}"
        );
    }
}

#[test]
fn a_card_that_never_said_rests_on_the_gba_cart() {
    let d = two_tetrises();
    let a = resumed(&d, None);
    let rom = a.browsed_rom().expect("the shelf rests on nothing");
    assert!(rom.ends_with("GBA/Tetris.gba"), "{rom:?}");
}

#[test]
fn a_named_shelf_that_no_longer_has_the_cart_leaves_the_shelf_at_its_start() {
    let d = two_tetrises();
    let a = resumed(&d, Some(Platform::Gbc));
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(
        a.selected_stem(),
        Some("Emerald"),
        "a Colour Tetris that is not on the card still moved the shelf"
    );
}

#[test]
fn putting_a_cart_in_records_the_shelf_it_came_off() {
    let d = two_tetrises();
    let mut a = boot(d.path());
    a.apply(Action::GbaDown(Btn::Down));
    assert_eq!(
        a.selected_stem(),
        Some("Tetris"),
        "down did not go to the Game Boy shelf"
    );
    insert(&mut a);
    settle(&mut a);
    assert!(a.handed_off().is_some(), "the cart was never handed off");
    let s = read_slot_state(d.path());
    assert_eq!(
        (s.cart.as_deref(), s.cart_platform),
        (Some("Tetris"), Some(Platform::Gb)),
        "the card does not say which Tetris was put in"
    );

    let again = App::boot(d.path());
    let rom = again.browsed_rom().expect("the shelf rests on nothing");
    assert!(rom.ends_with("GB/Tetris.gb"), "{rom:?}");
}
