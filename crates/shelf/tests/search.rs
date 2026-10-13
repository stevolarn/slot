mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::{App, Phase};
use shelf_input::{Action, Btn};
use shelf_store::{write_shelf_state, ShelfState};
use shelf_ui::QuickRow;

fn boot() -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Advance Wars", "Golden Sun", "Metroid Fusion", "Wario Ware"]);
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            ..ShelfState::default()
        },
    )
    .unwrap();
    let (a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    (d, a)
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn open_search(a: &mut App) {
    a.apply(Action::QuickMenu);
    while a.quick_menu() != Some(QuickRow::Search) {
        press(a, Btn::Down);
    }
    press(a, Btn::A);
    assert!(matches!(a.phase(), Phase::Search { .. }), "{:?}", a.phase());
}

/// Up to `letter` from the blank, then Right to keep it.
fn spell(a: &mut App, word: &str) {
    for ch in word.chars() {
        let steps = " ABCDEFGHIJKLMNOPQRSTUVWXYZ".find(ch).unwrap();
        for _ in 0..steps {
            press(a, Btn::Up);
        }
        press(a, Btn::Right);
    }
}

#[test]
fn spelling_narrows_the_matches_and_a_goes_to_the_game() {
    let (_d, mut a) = boot();
    open_search(&mut a);
    let lines = a.screen_text().unwrap().lines;
    assert_eq!(lines.last().map(String::as_str), Some("Up and Down pick a letter, Right adds another"));
    assert!(lines.iter().any(|l| l == "4 games"), "{lines:?}");
    spell(&mut a, "WAR");
    let lines = a.screen_text().unwrap().lines;
    assert_eq!(lines[0], "WAR_");
    assert!(lines.iter().any(|l| l == "2 games"), "{lines:?}");
    // Wario Ware starts with the letters, so it comes before Advance Wars.
    assert!(lines[1].starts_with("> Wario Ware"), "{lines:?}");
    press(&mut a, Btn::R1);
    press(&mut a, Btn::A);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(a.selected_stem(), Some("Advance Wars"));
}

#[test]
fn b_takes_letters_back_and_closes_once_nothing_is_spelt() {
    let (_d, mut a) = boot();
    open_search(&mut a);
    spell(&mut a, "G");
    press(&mut a, Btn::B);
    assert_eq!(a.screen_text().unwrap().lines[0], "[G]");
    press(&mut a, Btn::B);
    assert_eq!(a.screen_text().unwrap().lines[0], "_");
    press(&mut a, Btn::B);
    assert!(!matches!(a.phase(), Phase::Search { .. }));
}
