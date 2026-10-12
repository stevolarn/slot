mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::{App, FaceKind, Phase};
use shelf_gfx::Draw;
use shelf_input::{Action, Btn};
use shelf_store::{write_shelf_state, DisplayMode, ShelfState};
use shelf_ui::TexId;

const STEMS: [&str; 5] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo"];

fn boot(state: ShelfState) -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&STEMS);
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            display: DisplayMode::Boxes,
            ..state
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

fn run(a: &mut App, frames: usize) {
    for _ in 0..frames {
        a.update(1.0 / 60.0);
    }
}

fn frame(a: &App) -> Vec<Draw> {
    let mut out = Vec::new();
    a.draw(&mut out);
    out
}

fn tex_rects(out: &[Draw], tex: TexId) -> Vec<(f32, f32, f32, f32)> {
    out.iter()
        .filter_map(|d| match *d {
            Draw::Tex {
                x, y, w, h, tex: t, ..
            } if t == tex => Some((x, y, w, h)),
            _ => None,
        })
        .collect()
}

const TALL: usize = 300;
const CART: usize = 400;

/// Give every game a box front for the box shelf and a cart face.
fn faces(a: &mut App) {
    for i in 0..STEMS.len() {
        a.set_tall_face(0, i, TexId::from_raw(TALL + i), 168, 240);
        a.set_cart_face(0, i, TexId::from_raw(CART + i));
    }
}

/// Open the middle box and wait for its cart to come right out.
fn peek_out(a: &mut App) {
    press(a, Btn::A);
    assert!(a.peek().is_some(), "A did not open the box");
    run(a, 60);
    assert!(a.peek().is_some_and(|p| p.taken_out(a.now())));
}

#[test]
fn the_box_shelf_draws_box_fronts_in_the_middle_and_either_side() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    let out = frame(&a);
    let middle = tex_rects(&out, TexId::from_raw(TALL));
    assert_eq!(middle.len(), 1, "the middle box is not drawn once");
    let (x, _, w, _) = middle[0];
    assert!((x + w / 2.0 - 360.0).abs() < 1.0, "the middle box is not centred");
    // Its neighbours, smaller.
    let right = tex_rects(&out, TexId::from_raw(TALL + 1));
    assert_eq!(right.len(), 1);
    assert!(right[0].2 < w, "the box beside it is as large");
    // No cart is drawn while browsing boxes.
    for i in 0..STEMS.len() {
        assert!(tex_rects(&out, TexId::from_raw(CART + i)).is_empty());
    }
}

#[test]
fn the_box_shelf_asks_for_tall_boxes_and_the_middle_cart_first() {
    let (_d, a) = boot(ShelfState::default());
    let wants = a.face_wants();
    assert_eq!(wants[0].kind, FaceKind::Cart, "the middle cart is not first");
    assert_eq!(wants[0].index, 0);
    assert!(wants[1..].iter().all(|w| w.kind == FaceKind::Tall));
}

#[test]
fn a_opens_the_box_and_the_cart_settles_where_the_cart_shelf_has_it() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    peek_out(&mut a);
    let out = frame(&a);
    let cart = tex_rects(&out, TexId::from_raw(CART));
    assert_eq!(cart.len(), 1, "the cart is not out");
    // Only that game: no boxes either side, and its box is put away.
    for i in 0..STEMS.len() {
        assert!(
            tex_rects(&out, TexId::from_raw(TALL + i)).is_empty(),
            "box {i} is still drawn"
        );
    }
    assert_eq!(a.display(), DisplayMode::Boxes, "the peek changed the display");
    assert!(matches!(a.phase(), Phase::Shelf));
}

#[test]
fn the_cart_coming_out_never_goes_over_the_name() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    press(&mut a, Btn::A);
    let name_foot = 14.0 + 32.0 + 20.0;
    for _ in 0..60 {
        run(&mut a, 1);
        for (_, y, _, _) in tex_rects(&frame(&a), TexId::from_raw(CART)) {
            assert!(y >= name_foot, "the cart rose to {y}, over the name");
        }
    }
}

#[test]
fn a_again_takes_the_slot_animation_and_hands_the_game_off() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    press(&mut a, Btn::A);
    // Pressed again before the cart is out, A does nothing.
    run(&mut a, 3);
    press(&mut a, Btn::A);
    assert!(matches!(a.phase(), Phase::Shelf));
    run(&mut a, 60);
    press(&mut a, Btn::A);
    run(&mut a, 2);
    assert!(matches!(a.phase(), Phase::Inserting { .. }));
    assert!(a.handed_off().is_none(), "handed off with no slot animation");
    run(&mut a, 240);
    let handed = a.handed_off().expect("the game was not handed off");
    assert!(handed.ends_with("Alpha.gba"), "{handed:?}");
}

#[test]
fn b_puts_the_cart_back_and_the_boxes_return() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    peek_out(&mut a);
    press(&mut a, Btn::B);
    run(&mut a, 60);
    assert!(a.peek().is_none(), "the box did not shut");
    let out = frame(&a);
    assert_eq!(tex_rects(&out, TexId::from_raw(TALL)).len(), 1);
    assert!(tex_rects(&out, TexId::from_raw(CART)).is_empty());
    assert_eq!(a.selected_stem(), Some("Alpha"));
}

#[test]
fn right_while_open_shuts_the_box_then_moves_on_one() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    peek_out(&mut a);
    a.apply(Action::GbaDown(Btn::Right));
    // The box shuts first, on the same game.
    run(&mut a, 2);
    assert_eq!(a.selected_stem(), Some("Alpha"));
    a.apply(Action::GbaUp(Btn::Right));
    run(&mut a, 60);
    assert!(a.peek().is_none());
    assert_eq!(a.selected_stem(), Some("Bravo"));
}

#[test]
fn x_does_nothing_on_the_box_shelf() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 10);
    press(&mut a, Btn::X);
    run(&mut a, 5);
    assert!(a.box_view().is_none() && a.peek().is_none());
    assert!(a.blow().is_none());
    assert!(!a.refusal_active(a.now()));
}

#[test]
fn b_turns_the_box_to_its_back_and_again_to_its_front() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    press(&mut a, Btn::B);
    run(&mut a, 60);
    let now = a.now();
    let peek = a.peek().expect("B did not take the box up");
    assert!(peek.on_back(now) && peek.at_rest(now));
    assert!(!a.animating(), "a box resting on its back asks for frames");
    // Its front is not drawn while its back shows.
    assert!(tex_rects(&frame(&a), TexId::from_raw(TALL)).is_empty());
    press(&mut a, Btn::B);
    run(&mut a, 60);
    assert!(a.peek().is_none(), "the box did not go back on the shelf");
    assert_eq!(tex_rects(&frame(&a), TexId::from_raw(TALL)).len(), 1);
}

#[test]
fn a_on_the_back_turns_the_box_round_and_opens_it() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    press(&mut a, Btn::B);
    run(&mut a, 60);
    press(&mut a, Btn::A);
    run(&mut a, 90);
    assert!(a.peek().is_some_and(|p| p.taken_out(a.now())), "the cart is not out");
    assert_eq!(tex_rects(&frame(&a), TexId::from_raw(CART)).len(), 1);
    // B now puts the cart back, and the box goes back on the shelf.
    press(&mut a, Btn::B);
    run(&mut a, 60);
    assert!(a.peek().is_none());
}

#[test]
fn l2_in_a_peek_leaves_it_for_the_store() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    peek_out(&mut a);
    press(&mut a, Btn::L2);
    assert_eq!(a.display(), DisplayMode::Store);
    assert!(a.peek().is_none());
}

/// Rects in the colour of rubbed print: shelf wear.
fn wear(out: &[Draw]) -> usize {
    out.iter()
        .filter(|d| match d {
            Draw::Rect { colour, .. } => {
                (colour[0] - 0.86).abs() < 1e-3 && (colour[1] - 0.84).abs() < 1e-3
            }
            _ => false,
        })
        .count()
}

#[test]
fn dusty_boxes_show_wear_and_clean_ones_none() {
    let (_d, mut a) = boot(ShelfState {
        dusty: true,
        ..ShelfState::default()
    });
    faces(&mut a);
    run(&mut a, 30);
    assert!(wear(&frame(&a)) > 0, "no wear on games never played");

    let (_d, mut a) = boot(ShelfState {
        dusty: false,
        ..ShelfState::default()
    });
    faces(&mut a);
    run(&mut a, 30);
    assert_eq!(wear(&frame(&a)), 0, "wear with Dusty Carts off");
}

#[test]
fn a_shut_box_is_cleared_without_waiting_for_a_press() {
    let (_d, mut a) = boot(ShelfState::default());
    faces(&mut a);
    run(&mut a, 30);
    peek_out(&mut a);
    assert!(!a.animating(), "an open peek at rest still asks for frames");
    press(&mut a, Btn::B);
    assert!(a.animating(), "a shutting box does not ask for frames");
    run(&mut a, 60);
    assert!(a.peek().is_none() && !a.animating());
    // So A opens it again straight away.
    press(&mut a, Btn::A);
    assert!(a.peek().is_some());
}
