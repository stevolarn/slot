mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::App;
use shelf_input::{Action, Btn};
use shelf_store::{read_shelf_state, write_rentals, write_shelf_state, Rental, Rentals, ShelfState};
use shelf_ui::QuickRow;

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

#[test]
fn about_shows_a_card_of_the_plays_and_the_first_one_found() {
    let d = tmp_root_with_carts(&["Alpha", "Bravo"]);
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            ..ShelfState::default()
        },
    )
    .unwrap();
    let mut r = Rentals::new();
    r.insert(d.path().join("Games/GBA/Alpha.gba"), Rental { times: 5, last: 1_700_000_000 });
    r.insert(d.path().join("Games/GBA/Bravo.gba"), Rental { times: 2, last: 1_600_000_000 });
    write_rentals(d.path(), &r).unwrap();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert_eq!(read_shelf_state(d.path()).member_since, 1_600_000_000, "not backdated");
    assert!(a.member_card().is_none(), "a card off the About screen");
    a.apply(Action::QuickMenu);
    while a.quick_menu() != Some(QuickRow::About) {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    let card = a.member_card().expect("no card on About");
    assert_eq!(card.plays, 7);
    assert!(card.plain);
    let rows = shelf_ui::member_rows(&card);
    assert_eq!(rows[0], ("MEMBER SINCE", "SEP 2020".to_string()));
    assert_eq!(rows[1], ("PLAYS", "7".to_string()));
}
