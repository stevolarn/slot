mod common;

use std::path::Path;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::App;
use shelf_store::{write_shelf_state, ShelfState};

fn write_in(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create dir");
    std::fs::write(&path, body).expect("write file");
}

#[test]
fn x_on_the_box_back_plays_a_saved_game_from_its_start() {
    fresh_from(shelf_store::DisplayMode::Boxes, false, 1);
    fresh_from(shelf_store::DisplayMode::Carts, false, 2);
    fresh_from(shelf_store::DisplayMode::Store, false, 1);
    fresh_from(shelf_store::DisplayMode::Store, true, 2);
}

/// `turns` presses of B to bring the box round to its back.
fn fresh_from(display: shelf_store::DisplayMode, store_carts: bool, turns: usize) {
    let roms = tempfile::tempdir().unwrap();
    write_in(roms.path(), "Game Boy Advance (GBA)/Minish Cap.gba", "rom");
    write_in(roms.path(), "Game Boy Advance (GBA)/.media/Minish Cap.png", "x");
    std::env::set_var(shelf::handoff::ROMS_VAR, roms.path());
    let shared = tempfile::tempdir().unwrap();
    write_in(shared.path(), ".minui/GBA/Minish Cap.gba.txt", "0");
    let root = tmp_root_with_carts(&[]);
    write_shelf_state(
        root.path(),
        &ShelfState {
            clock_set: true,
            display,
            store_carts,
            ..ShelfState::default()
        },
    )
    .unwrap();
    std::env::set_var(shelf::handoff::SHARED_USERDATA_VAR, shared.path());
    let (mut a, _) = app_booting_at(root.path(), CLOCK_IS_SET);
    let run = |a: &mut App, n: usize| (0..n).for_each(|_| a.update(1.0 / 60.0));
    let press = |a: &mut App, b: shelf_input::Btn| {
        a.apply(shelf_input::Action::GbaDown(b));
        a.apply(shelf_input::Action::GbaUp(b));
    };
    run(&mut a, 10);
    assert!(a.selected_saved());
    if display == shelf_store::DisplayMode::Boxes {
        press(&mut a, shelf_input::Btn::X);
        assert!(matches!(a.phase(), shelf::app::Phase::Shelf), "X on the box front did something");
    }
    for _ in 0..turns {
        press(&mut a, shelf_input::Btn::B);
        run(&mut a, 60);
    }
    press(&mut a, shelf_input::Btn::X);
    assert!(
        matches!(a.phase(), shelf::app::Phase::Inserting { .. }),
        "{display:?} {store_carts}: {:?}",
        a.phase()
    );
    assert!(!a.resume_saves(), "the game would load its save state");
}
