mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::App;
use shelf_input::{Action, Btn};
use shelf_store::{write_shelf_state, ShelfState};
use shelf_ui::QuickRow;

const STEMS: [&str; 5] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo"];

fn boot(d: &tempfile::TempDir, show_hidden: bool) -> App {
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            show_hidden,
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

/// Menu, up to Hide Game, A.
fn hide_from_menu(a: &mut App) {
    a.apply(Action::QuickMenu);
    while a.quick_menu() != Some(QuickRow::Hide) {
        press(a, Btn::Up);
    }
    press(a, Btn::A);
}

fn stems(a: &App) -> Vec<String> {
    a.carts().map(|c| c.stem.clone()).collect()
}

#[test]
fn hide_game_takes_the_game_off_the_shelf_and_keeps_it_off_after_a_restart() {
    let d = tmp_root_with_carts(&STEMS);
    let mut a = boot(&d, false);
    assert_eq!(a.selected_stem(), Some("Alpha"));
    hide_from_menu(&mut a);
    assert_eq!(a.quick_menu(), None, "the menu closes");
    assert_eq!(a.selected_stem(), Some("Bravo"));
    assert!(!stems(&a).contains(&"Alpha".to_string()));
    let hidden = std::fs::read_to_string(d.path().join("Config/hidden.txt")).unwrap();
    assert!(hidden.contains("Alpha"), "{hidden}");

    let a = boot(&d, false);
    assert_eq!(stems(&a), ["Bravo", "Charlie", "Delta", "Echo"]);
}

#[test]
fn shown_hidden_games_come_back_and_hide_game_then_unhides_them() {
    let d = tmp_root_with_carts(&STEMS);
    let mut a = boot(&d, false);
    hide_from_menu(&mut a);

    let mut a = boot(&d, true);
    assert_eq!(stems(&a).len(), 5);
    assert_eq!(a.selected_stem(), Some("Alpha"));
    assert!(a.selected_hidden());
    hide_from_menu(&mut a);
    assert!(!a.selected_hidden());

    let a = boot(&d, false);
    assert_eq!(stems(&a).len(), 5, "Alpha is no longer hidden");
}

#[test]
fn show_hidden_in_options_puts_hidden_games_back_at_once() {
    let d = tmp_root_with_carts(&STEMS);
    let mut a = boot(&d, false);
    hide_from_menu(&mut a);
    assert_eq!(stems(&a).len(), 4);
    a.apply(Action::QuickMenu);
    while a.quick_menu() != Some(QuickRow::Options) {
        press(&mut a, Btn::Up);
    }
    press(&mut a, Btn::A);
    while a.quick_menu() != Some(QuickRow::ShowHidden) {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::Right);
    assert_eq!(stems(&a).len(), 5);
    assert_eq!(
        a.selected_stem(),
        Some("Bravo"),
        "the shelf stays on its game"
    );
}
