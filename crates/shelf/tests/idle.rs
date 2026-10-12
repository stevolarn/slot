use shelf::app::App;
use shelf::idle::Idle;
use shelf_input::{Action, Btn, Gestures, RawEvent};
use shelf_store::{Cart, Platform};

fn app(stems: &[&str]) -> App {
    App::new(
        stems
            .iter()
            .map(|stem| Cart {
                platform: Platform::Gba,
                stem: (*stem).to_string(),
                rom: format!("Games/GBA/{stem}.gba").into(),
                label: None,
                code: String::new(),
                shell: None,
                title: stem.to_uppercase(),
            })
            .collect(),
    )
}

#[test]
fn a_screen_is_not_at_rest_until_several_frames_in_a_row_matched() {
    let mut idle = Idle::default();
    assert!(!idle.rested());
    for _ in 0..3 {
        idle.skipped();
    }
    assert!(!idle.rested(), "three matching frames were enough");
    idle.skipped();
    assert!(idle.rested());
    idle.drawn();
    assert!(!idle.rested(), "a drawn frame did not end the rest");
    assert_eq!(idle.tick(), shelf::idle::REST_TICK);
    for _ in 0..100 {
        idle.skipped();
    }
    assert_eq!(idle.tick(), shelf::idle::LONG_REST_TICK, "a long rest still woke as often");
    idle.drawn();
    assert_eq!(idle.tick(), shelf::idle::REST_TICK, "a drawn frame kept the long wait");
}

#[test]
fn a_resting_shelf_is_not_animating_but_a_held_direction_is() {
    let mut a = app(&["A", "B", "C", "D"]);
    a.update(1.0 / 60.0);
    assert!(!a.animating(), "a shelf at rest is animating");
    a.apply(Action::GbaDown(Btn::Right));
    assert!(a.animating(), "a held direction waiting to repeat is not");
    a.apply(Action::GbaUp(Btn::Right));
    assert!(!a.animating(), "a released direction still is");
}

#[test]
fn a_held_play_button_keeps_the_app_animating_until_it_is_let_go() {
    let mut a = app(&["A", "B"]);
    a.apply(Action::GbaDown(Btn::A));
    assert!(a.animating(), "A held down can still turn into a blow");
    a.apply(Action::GbaUp(Btn::A));
}

#[test]
fn gestures_report_a_pending_hold_until_it_is_over() {
    let mut g = Gestures::new();
    assert!(!g.pending());
    g.feed(RawEvent::Down(Btn::Power), 0);
    assert!(g.pending(), "a held power button has a hold to fire");
    g.feed(RawEvent::Up(Btn::Power), 100);
    assert!(!g.pending());
    g.feed(RawEvent::Down(Btn::VolUp), 200);
    assert!(g.pending(), "a held volume key has a repeat to fire");
    g.feed(RawEvent::Up(Btn::VolUp), 300);
    assert!(!g.pending());
    g.feed(RawEvent::Down(Btn::A), 400);
    assert!(!g.pending(), "A is the app's to track, not the gestures'");
}

#[test]
fn nextui_screen_timeout_is_read_in_milliseconds_and_zero_is_never() {
    use shelf::handoff::screen_timeout_in;
    assert_eq!(screen_timeout_in("font=1\nscreentimeout=600\n"), Some(600_000));
    assert_eq!(screen_timeout_in("screentimeout=0\n"), None);
    assert_eq!(screen_timeout_in("font=1\n"), None);
}

#[test]
fn with_no_button_pressed_for_the_screen_timeout_it_goes_to_sleep() {
    let mut a = app(&["Alpha", "Bravo"]);
    a.set_screen_timeout(Some(2_000));
    for _ in 0..60 {
        a.update(1.0 / 60.0);
    }
    assert!(!a.take_sleep(), "asleep before the timeout");
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaUp(Btn::Right));
    for _ in 0..90 {
        a.update(1.0 / 60.0);
    }
    assert!(!a.take_sleep(), "a press did not start the wait again");
    for _ in 0..40 {
        a.update(1.0 / 60.0);
    }
    assert!(a.take_sleep(), "it did not sleep after the timeout");
    // Time asleep does not count towards the next timeout.
    a.update(3600.0);
    assert!(!a.take_sleep(), "it slept again straight after waking");
}
