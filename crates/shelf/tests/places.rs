mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf_store::{read_shelf_state, write_shelf_state, Platform, ShelfState};

const STEMS: [&str; 5] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo"];

#[test]
fn a_console_shelf_opens_on_the_game_it_was_left_on() {
    let d = tmp_root_with_carts(&STEMS);
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            places: vec![(Platform::Gba, "Delta".into())],
            ..ShelfState::default()
        },
    )
    .unwrap();
    let (a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert_eq!(a.selected_stem(), Some("Delta"));
}

#[test]
fn a_place_whose_game_is_gone_leaves_the_shelf_at_its_start() {
    let d = tmp_root_with_carts(&STEMS);
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            places: vec![(Platform::Gba, "Zulu".into())],
            ..ShelfState::default()
        },
    )
    .unwrap();
    let (a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert_eq!(a.selected_stem(), Some("Alpha"));
    assert_eq!(read_shelf_state(d.path()).places.len(), 1);
}
