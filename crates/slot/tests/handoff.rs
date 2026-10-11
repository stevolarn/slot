mod common;

use common::{session_with_platform, tmp_root_with_carts};
use slot::app::Phase;
use slot::session::Session;
use slot_input::{Btn, Millis, RawEvent};

fn handing_off() {
    std::env::set_var(slot::handoff::HANDOFF_VAR, "/tmp/slot-handoff-test");
}

fn step(s: &mut Session, now: &mut Millis, evs: Vec<RawEvent>) {
    *now += 16;
    s.feed(evs, *now);
    s.update(1.0 / 60.0);
}

fn press(s: &mut Session, now: &mut Millis, btn: Btn) {
    step(s, now, vec![RawEvent::Down(btn)]);
    step(s, now, vec![RawEvent::Up(btn)]);
}

#[test]
fn a_cart_put_in_is_handed_off_once_it_is_seated_and_no_core_runs() {
    handing_off();
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let (mut s, _) = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    assert!(s.app().handoff());
    let chosen = s
        .app()
        .browsed_rom()
        .expect("no cart on the shelf")
        .to_path_buf();
    press(&mut s, &mut now, Btn::A);
    assert_eq!(
        s.app().handed_off(),
        None,
        "handed off before it was seated"
    );
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    assert_eq!(s.app().handed_off(), Some(chosen.as_path()));
    assert!(!s.has_core(), "a core ran in game picker mode");
}

#[test]
fn coming_back_rests_on_the_cart_that_was_played_without_putting_it_in() {
    handing_off();
    let d = tmp_root_with_carts(&["Advance Wars", "Emerald"]);
    let (mut s, _) = session_with_platform(d.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    press(&mut s, &mut now, Btn::Right);
    for _ in 0..30 {
        step(&mut s, &mut now, vec![]);
    }
    let chosen = s.app().browsed_rom().expect("no cart").to_path_buf();
    press(&mut s, &mut now, Btn::A);
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    assert_eq!(s.app().handed_off(), Some(chosen.as_path()));
    s.app().flush_state();
    drop(s);

    let (mut back, _) = session_with_platform(d.path());
    let mut now: Millis = 0;
    for _ in 0..60 {
        step(&mut back, &mut now, vec![]);
    }
    assert!(matches!(back.app().phase(), Phase::Shelf));
    assert_eq!(back.app().browsed_rom(), Some(chosen.as_path()));
    assert_eq!(
        back.app().handed_off(),
        None,
        "handed the same game off again"
    );
}
