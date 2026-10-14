mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::App;
use shelf_input::{Action, Btn};
use shelf_store::{write_shelf_state, ShelfState};
use shelf_ui::{QuickRow, QuickValue};

fn boot(d: &tempfile::TempDir) -> App {
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            ..ShelfState::default()
        },
    )
    .unwrap();
    app_booting_at(d.path(), CLOCK_IS_SET).0
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

#[test]
fn start_opens_a_menu_of_favourite_hide_and_emulator_for_the_game() {
    let d = tmp_root_with_carts(&["Alpha", "Bravo"]);
    let mut a = boot(&d);
    press(&mut a, Btn::Start);
    assert_eq!(a.quick_menu(), Some(QuickRow::Favourite));
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Hide));
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::HideConsole));
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Emulator));
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Favourite), "down did not wrap");
    press(&mut a, Btn::B);
    assert_eq!(a.quick_menu(), None, "B did not close the game menu");
}

#[test]
fn the_favourite_row_adds_and_removes_the_game_on_either_arrow_or_a() {
    let d = tmp_root_with_carts(&["Alpha", "Bravo"]);
    let mut a = boot(&d);
    press(&mut a, Btn::Start);
    assert_eq!(a.quick_value(QuickRow::Favourite), Some(QuickValue::Off));
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::Favourite), Some(QuickValue::On));
    press(&mut a, Btn::Left);
    assert_eq!(a.quick_value(QuickRow::Favourite), Some(QuickValue::Off));
    press(&mut a, Btn::A);
    assert_eq!(a.quick_value(QuickRow::Favourite), Some(QuickValue::On));
    assert_eq!(a.favourites().len(), 1);
}

#[test]
fn the_emulator_row_refuses_a_console_with_one_emulator() {
    let d = tmp_root_with_carts(&["Alpha"]);
    let mut a = boot(&d);
    press(&mut a, Btn::Start);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Emulator));
    press(&mut a, Btn::Right);
    assert!(a.refusal_active(a.now()), "no refusal with nothing to choose");
}

fn platform_of(a: &App) -> Option<shelf_store::Platform> {
    let rom = a.browsed_rom()?;
    a.carts().find(|c| c.rom == rom).map(|c| c.platform)
}

fn two_consoles() -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Alpha", "Bravo"]);
    common::write_gb_cart(&d, "Zed", "ZED");
    d
}

fn hide_console(a: &mut App) {
    press(a, Btn::Start);
    while a.quick_menu() != Some(QuickRow::HideConsole) {
        press(a, Btn::Down);
    }
    press(a, Btn::A);
}

#[test]
fn hide_console_takes_the_whole_console_off_the_shelves_and_keeps_it_off() {
    let d = two_consoles();
    let mut a = boot(&d);
    let platform = platform_of(&a).unwrap();
    hide_console(&mut a);
    assert_eq!(a.quick_menu(), None, "the menu stayed open");
    assert!(
        a.carts().all(|c| c.platform != platform),
        "the console's games are still on the shelves"
    );
    assert!(a.carts().count() > 0, "no console is left");
    let again = boot(&d);
    assert!(
        again.carts().all(|c| c.platform != platform),
        "the console came back after a restart"
    );
}

#[test]
fn the_last_console_cannot_be_hidden() {
    let d = tmp_root_with_carts(&["Alpha", "Bravo"]);
    let mut a = boot(&d);
    hide_console(&mut a);
    assert_eq!(a.carts().count(), 2, "the only console was hidden");
    assert!(a.refusal_active(a.now()));
}

#[test]
fn a_hidden_console_comes_back_with_show_hidden_and_reads_unhide_console() {
    let d = two_consoles();
    let mut a = boot(&d);
    let platform = platform_of(&a).unwrap();
    hide_console(&mut a);
    drop(a);
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            show_hidden: true,
            ..ShelfState::default()
        },
    )
    .unwrap();
    let mut a = app_booting_at(d.path(), CLOCK_IS_SET).0;
    assert!(a.carts().any(|c| c.platform == platform), "not shown again");
    while platform_of(&a) != Some(platform) {
        a.apply(Action::GbaDown(Btn::Up));
        a.apply(Action::GbaUp(Btn::Up));
    }
    assert!(a.selected_console_hidden());
    hide_console(&mut a);
    assert!(!a.selected_console_hidden(), "Unhide Console did not unhide");
}
