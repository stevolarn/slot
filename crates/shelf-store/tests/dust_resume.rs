mod common;

use common::tmp_root;
use shelf_store::{read_shelf_state, write_shelf_state, ShelfState};

#[test]
fn dusty_carts_and_resume_saves_default_on_and_round_trip() {
    let fresh = ShelfState::default();
    assert!(fresh.dusty && fresh.resume);
    let d = tmp_root();
    let s = ShelfState {
        clock_set: true,
        dusty: false,
        resume: false,
        ..ShelfState::default()
    };
    write_shelf_state(d.path(), &s).unwrap();
    assert_eq!(read_shelf_state(d.path()), s);
    let text = std::fs::read_to_string(d.path().join("Config/shelf.state")).unwrap();
    for line in ["dusty=0", "resume=0"] {
        assert!(text.lines().any(|l| l == line), "no {line} in {text:?}");
    }
}

#[test]
fn a_state_file_from_before_dust_and_resume_loads_with_both_on() {
    let d = tmp_root();
    std::fs::write(
        d.path().join("Config/shelf.state"),
        "cart=Emerald\nbrightness=3\nvolume=40\nmuted=0\nclock_set=1\nutc_offset_min=0\ncrt=1\n",
    )
    .unwrap();
    let s = read_shelf_state(d.path());
    assert!(s.crt && s.dusty && s.resume);
    assert_eq!(s.cart.as_deref(), Some("Emerald"));
}
