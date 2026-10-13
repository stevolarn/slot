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
    assert_eq!(a.quick_menu(), Some(QuickRow::Emulator));
    press(&mut a, Btn::Right);
    assert!(a.refusal_active(a.now()), "no refusal with nothing to choose");
}
