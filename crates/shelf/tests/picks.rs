mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::{App, ShelfKey};
use shelf_input::{Action, Btn};
use shelf_store::{write_shelf_state, ShelfState};

fn card() -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Alpha", "Bravo", "Charlie", "Delta", "Echo"]);
    let dir = d.path().join("Details").join("GBA");
    std::fs::create_dir_all(&dir).unwrap();
    for (stem, rating) in [("Alpha", "2.0"), ("Charlie", "4.6"), ("Delta", "3.9")] {
        let text = format!("name={stem}\ngenre=Action\nrating={rating}\n");
        std::fs::write(dir.join(format!("{stem}.txt")), text).unwrap();
    }
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            ..ShelfState::default()
        },
    )
    .unwrap();
    d
}

/// Run until the details thread has finished and the picks are chosen.
fn settle(a: &mut App) {
    for _ in 0..500 {
        a.update(1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
fn the_best_rated_games_never_played_are_the_picks_above_the_first_console() {
    let d = card();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    settle(&mut a);
    assert_eq!(a.shelf_key(), ShelfKey::Platform(shelf_store::Platform::Gba));
    a.apply(Action::GbaDown(Btn::Up));
    assert_eq!(a.shelf_key(), ShelfKey::Picks, "Up did not reach the picks");
    assert_eq!(a.shelf_platform_name(), Some("Picked for You"));
    let mut seen = vec![a.selected_stem().unwrap().to_string()];
    a.apply(Action::GbaDown(Btn::Right));
    for _ in 0..60 {
        a.update(1.0 / 60.0);
    }
    seen.push(a.selected_stem().unwrap().to_string());
    seen.sort();
    assert_eq!(seen, ["Charlie", "Delta"], "a game rated under 3.5 or unrated was picked");
}
