mod common;

use common::{boot, tmp_root_with_carts};
use slot::app::App;
use slot_input::{Action, Btn};

#[test]
fn no_plate_is_drawn_for_a_refusal() {
    let d = tmp_root_with_carts(&["Emerald", "Zzz"]);
    let mut a = boot(d.path());
    a.refuse();
    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(a.refusal_active(a.now()), "nothing was refused");
    assert_eq!(
        out.len(),
        draws_without_refusal(&d),
        "a refusal added something to the screen"
    );
}

#[test]
fn the_shake_decays_and_stops() {
    let d = tmp_root_with_carts(&["Emerald", "Zzz"]);
    let mut a = boot(d.path());
    a.refuse();
    let early = peak(&a, 0, 100);
    let late = peak(&a, 200, 300);
    assert!(early > 0.0, "nothing moved");
    assert!(
        late < early,
        "the shake is not decaying: {early} then {late}"
    );
    assert_eq!(a.shake_at(a.now() + 400), 0.0, "the shake never ends");
}

#[test]
fn a_random_game_with_nothing_else_on_the_shelf_shakes_it() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::GbaDown(Btn::Select));
    assert!(a.refusal_active(a.now()), "nothing told the player why");
    assert_ne!(a.shelf_shake(), 0.0, "the carts did not flinch");
}

#[test]
fn a_shelf_refusal_shakes_the_carts() {
    let d = tmp_root_with_carts(&["Emerald", "Zzz"]);
    let mut a = boot(d.path());
    a.refuse();
    assert_ne!(a.shelf_shake(), 0.0, "nothing moved at all");
}

#[test]
fn the_shelf_only_shakes_while_it_is_showing() {
    let d = tmp_root_with_carts(&["Emerald", "Zzz"]);
    let mut a = boot(d.path());
    a.refuse();
    a.apply(Action::QuickMenu);
    assert_eq!(a.shelf_shake(), 0.0, "the menu shook with the shelf");
}

fn peak(a: &App, from: u64, to: u64) -> f32 {
    (from..to)
        .map(|t| a.shake_at(a.now() + t).abs())
        .fold(0.0, f32::max)
}

fn draws_without_refusal(d: &tempfile::TempDir) -> usize {
    let a = boot(d.path());
    let mut out = Vec::new();
    a.draw(&mut out);
    out.len()
}
