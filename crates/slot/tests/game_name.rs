mod common;

use common::{session_with_platform, tmp_root_with_carts};
use slot::session::Session;
use slot_gfx::{Draw, TexId};
use slot_input::{Btn, Millis, RawEvent};

fn step(s: &mut Session, now: &mut Millis, evs: Vec<RawEvent>) {
    *now += 16;
    s.feed(evs, *now);
    s.update(1.0 / 60.0);
}

fn drawn_name(s: &Session, tex: TexId) -> Option<(f32, f32)> {
    let mut out = Vec::new();
    s.app().draw(&mut out);
    out.iter().find_map(|d| match *d {
        Draw::Tex { tex: t, y, h, .. } if t == tex => Some((y, h)),
        _ => None,
    })
}

#[test]
fn the_middle_cart_has_its_name_printed_in_the_space_above_it() {
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    for _ in 0..90 {
        step(&mut s, &mut now, vec![]);
    }
    let want = s.app().wanted_game_name().expect("no name on the shelf");
    let face = TexId::from_raw(4242);
    s.app_mut().set_game_name_face(want.clone(), face);
    let (y, h) = drawn_name(&s, face).expect("the name was not drawn");
    let top = slot_ui::rest_y(slot_ui::CART_H as f32);
    assert!(
        y >= 0.0 && y + h <= top,
        "the name at {y} overlaps the cart top at {top}"
    );

    step(&mut s, &mut now, vec![RawEvent::Down(Btn::Right)]);
    step(&mut s, &mut now, vec![RawEvent::Up(Btn::Right)]);
    for _ in 0..30 {
        step(&mut s, &mut now, vec![]);
    }
    let next = s.app().wanted_game_name().expect("no name");
    assert_ne!(next, want, "the name did not follow the cart");
    assert!(drawn_name(&s, face).is_none(), "the old name stayed up");
}

#[test]
fn names_read_like_titles() {
    assert_eq!(
        slot_ui::game_name("Legend of Zelda, The - The Minish Cap (Europe) (En,Fr,De,Es,It)"),
        "The Legend of Zelda - The Minish Cap"
    );
    assert_eq!(
        slot_ui::game_name("Sonic the Hedgehog (USA, Europe)"),
        "Sonic the Hedgehog"
    );
    assert_eq!(
        slot_ui::game_name("Lost Vikings, The [T-En]"),
        "The Lost Vikings"
    );
}
