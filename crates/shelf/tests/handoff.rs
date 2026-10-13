mod common;

use common::{session_with_platform, tmp_root_with_carts};
use shelf::app::Phase;
use shelf::session::Session;
use shelf_input::{Btn, Millis, RawEvent};

fn step(s: &mut Session, now: &mut Millis, evs: Vec<RawEvent>) {
    *now += 16;
    s.feed(evs, *now);
    s.update(1.0 / 60.0);
}

fn press(s: &mut Session, now: &mut Millis, btn: Btn) {
    step(s, now, vec![RawEvent::Down(btn)]);
    step(s, now, vec![RawEvent::Up(btn)]);
}

#[test]
fn a_cart_put_in_is_handed_off_once_it_is_seated() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    let chosen = s
        .app()
        .browsed_rom()
        .expect("no cart on the shelf")
        .to_path_buf();
    press(&mut s, &mut now, Btn::A);
    assert_eq!(
        s.app().handed_off(),
        None,
        "handed off before it was seated"
    );
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    assert_eq!(s.app().handed_off(), Some(chosen.as_path()));
}

#[test]
fn coming_back_rests_on_the_cart_that_was_played_without_putting_it_in() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    press(&mut s, &mut now, Btn::Right);
    for _ in 0..30 {
        step(&mut s, &mut now, vec![]);
    }
    let chosen = s.app().browsed_rom().expect("no cart").to_path_buf();
    press(&mut s, &mut now, Btn::A);
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    assert_eq!(s.app().handed_off(), Some(chosen.as_path()));
    s.app().flush_state();
    drop(s);

    let mut back = session_with_platform(d.path());
    let mut now: Millis = 0;
    for _ in 0..60 {
        step(&mut back, &mut now, vec![]);
    }
    assert!(matches!(back.app().phase(), Phase::Shelf));
    assert_eq!(back.app().browsed_rom(), Some(chosen.as_path()));
    assert_eq!(
        back.app().handed_off(),
        None,
        "handed the same game off again"
    );
}

#[test]
fn start_on_the_shelf_opens_the_button_guide_and_start_closes_it() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    press(&mut s, &mut now, Btn::Start);
    assert!(s.app().help_open(), "Start did not open the guide");
    assert_eq!(s.app().quick_menu(), None, "Start opened the menu");
    press(&mut s, &mut now, Btn::Start);
    assert!(!s.app().help_open(), "Start did not close the guide");
    assert!(matches!(s.app().phase(), Phase::Shelf));
}

#[test]
fn exit_on_the_menu_leaves_without_a_game_or_a_power_off() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    press(&mut s, &mut now, Btn::Menu);
    for _ in 0..shelf_ui::QuickRow::Exit.position() {
        press(&mut s, &mut now, Btn::Down);
    }
    assert_eq!(s.app().quick_menu(), Some(shelf_ui::QuickRow::Exit));
    press(&mut s, &mut now, Btn::A);
    assert!(s.app().exiting(), "Exit did not ask to leave");
    assert_eq!(s.app().handed_off(), None, "Exit handed a game off");
    assert!(!s.app().shutting_down(), "Exit powered the Brick down");
}

#[test]
fn menu_on_the_shelf_opens_the_settings_and_b_goes_back_to_the_shelf() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    press(&mut s, &mut now, Btn::Menu);
    assert_eq!(
        s.app().quick_menu(),
        Some(shelf_ui::QuickRow::Options),
        "MENU did not open the menu: {:?}",
        s.app().phase()
    );
    press(&mut s, &mut now, Btn::B);
    assert!(matches!(s.app().phase(), Phase::Shelf));
}

#[test]
fn about_is_a_row_of_the_settings_and_b_comes_back_to_it() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    press(&mut s, &mut now, Btn::Menu);
    press(&mut s, &mut now, Btn::Down);
    press(&mut s, &mut now, Btn::Down);
    press(&mut s, &mut now, Btn::Down);
    assert_eq!(s.app().quick_menu(), Some(shelf_ui::QuickRow::About));
    press(&mut s, &mut now, Btn::A);
    assert!(
        matches!(s.app().phase(), Phase::About),
        "{:?}",
        s.app().phase()
    );
    press(&mut s, &mut now, Btn::B);
    assert_eq!(s.app().quick_menu(), Some(shelf_ui::QuickRow::About));
}

#[test]
fn picker_mode_makes_no_save_state_or_bios_folders() {
    let d = tempfile::tempdir().expect("tempdir");
    shelf::root::ensure(d.path());
    for gone in ["BIOS", "Games", "Saves", "States", "Labels", "Wallpapers"] {
        assert!(!d.path().join(gone).exists(), "made {gone}");
    }
    assert!(d.path().join("Config").is_dir());
    assert!(d.path().join("System").is_dir());
}

#[test]
fn a_game_played_comes_back_on_the_recently_played_shelf() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    press(&mut s, &mut now, Btn::Right);
    for _ in 0..30 {
        step(&mut s, &mut now, vec![]);
    }
    let chosen = s.app().browsed_rom().expect("no cart").to_path_buf();
    press(&mut s, &mut now, Btn::A);
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    s.app().flush_state();
    drop(s);

    let mut back = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut back, &mut now, vec![]);
    press(&mut back, &mut now, Btn::Up);
    assert_eq!(back.app().shelf_key(), shelf::app::ShelfKey::Recent);
    assert_eq!(back.app().browsed_rom(), Some(chosen.as_path()));
}
