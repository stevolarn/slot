use shelf_input::RawEvent::{Down, Up};
use shelf_input::{
    Action::*, Btn, Btn::*, Gestures, RawEvent, POWER_HOLD_MS, SELECT_CHORD_MS, SELECT_TAP_MS,
    VOLUME_REPEAT_DELAY_MS, VOLUME_REPEAT_MS,
};

#[test]
fn a_select_that_becomes_a_chord_never_reaches_the_game() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Select), 0).is_empty());
    assert_eq!(g.feed(Down(Btn::Up), 120), vec![BrightnessUp]);
    assert!(g.feed(Up(Btn::Up), 160).is_empty());
    assert!(g.feed(Up(Select), 900).is_empty());
    assert!(g.tick(5_000).is_empty());
}

#[test]
fn a_select_held_past_the_window_reaches_the_game_then() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Select), 0).is_empty());
    assert!(g.tick(SELECT_CHORD_MS - 1).is_empty());
    assert_eq!(g.tick(SELECT_CHORD_MS), vec![GbaDown(Select)]);
    assert!(g.tick(SELECT_CHORD_MS + 100).is_empty());
    assert_eq!(g.feed(Up(Select), 2_000), vec![GbaUp(Select)]);
}

#[test]
fn a_game_button_under_select_hands_select_over_first() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Select), 0).is_empty());
    assert_eq!(
        g.feed(Down(Start), 100),
        vec![GbaDown(Select), GbaDown(Start)]
    );
    assert_eq!(g.feed(Up(Start), 200), vec![GbaUp(Start)]);
    assert!(
        g.tick(SELECT_CHORD_MS).is_empty(),
        "SELECT was pressed twice"
    );
    assert_eq!(g.feed(Up(Select), 900), vec![GbaUp(Select)]);
}

#[test]
fn a_held_select_keeps_chording_long_past_the_window() {
    let mut g = Gestures::new();
    g.feed(Down(Select), 0);
    assert_eq!(g.feed(Down(Btn::Up), 100), vec![BrightnessUp]);
    g.feed(Up(Btn::Up), 150);
    assert_eq!(
        g.feed(Down(Btn::Up), SELECT_CHORD_MS + 500),
        vec![BrightnessUp],
        "the second nudge of the brightness reached the game as a direction"
    );
}

#[test]
fn a_select_tap_reaches_the_game_on_its_release() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Select), 0).is_empty());
    assert_eq!(g.feed(Up(Select), 80), vec![GbaDown(Select)]);
    assert!(g.tick(80 + SELECT_TAP_MS - 1).is_empty());
    assert_eq!(g.tick(80 + SELECT_TAP_MS), vec![GbaUp(Select)]);
    assert!(g.tick(5_000).is_empty());
}

#[test]
fn a_select_press_inside_the_tap_window_hands_back_the_release_it_interrupted() {
    let mut g = Gestures::new();
    let mut log = Vec::new();
    log.extend(g.feed(Down(Select), 0));
    log.extend(g.feed(Up(Select), 20));
    log.extend(g.feed(Down(Select), 20 + SELECT_TAP_MS - 1));
    log.extend(g.feed(Down(Btn::Up), 100));
    log.extend(g.feed(Up(Btn::Up), 140));
    log.extend(g.feed(Up(Select), 200));
    for t in 200..2_000 {
        log.extend(g.tick(t));
    }
    let downs = log.iter().filter(|a| **a == GbaDown(Select)).count();
    let ups = log.iter().filter(|a| **a == GbaUp(Select)).count();
    assert_eq!(
        (downs, ups),
        (1, 1),
        "the core was handed {downs} SELECT press(es) and {ups} release(s): {log:?}"
    );
    assert_eq!(
        log.iter()
            .rfind(|a| matches!(a, GbaDown(Select) | GbaUp(Select))),
        Some(&GbaUp(Select)),
        "the pad was left holding SELECT: {log:?}"
    );
    assert!(
        log.contains(&BrightnessUp),
        "the chord off the second press stopped working: {log:?}"
    );
}

#[test]
fn past_the_window_select_and_the_next_key_are_the_games() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Select), 0).is_empty());
    assert!(
        g.tick(400).is_empty(),
        "SELECT was handed over inside its window"
    );
    assert_eq!(g.tick(SELECT_CHORD_MS), vec![GbaDown(Select)]);
    assert_eq!(
        g.feed(Down(Btn::Up), SELECT_CHORD_MS + 1),
        vec![GbaDown(Btn::Up)]
    );
}

#[test]
fn a_select_tap_too_short_to_be_polled_is_held_on_to() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Select), 0).is_empty());
    assert_eq!(g.feed(Up(Select), 20), vec![GbaDown(Select)]);
    assert!(g.tick(20 + SELECT_TAP_MS - 1).is_empty());
    assert_eq!(g.tick(20 + SELECT_TAP_MS), vec![GbaUp(Select)]);
    assert!(
        g.tick(5_000).is_empty(),
        "the release was handed over twice"
    );
}

#[test]
fn a_select_the_game_has_seen_ends_on_its_own_release() {
    let mut g = Gestures::new();
    g.feed(Down(Select), 0);
    assert_eq!(g.tick(SELECT_CHORD_MS), vec![GbaDown(Select)]);
    assert_eq!(
        g.feed(Up(Select), SELECT_CHORD_MS + SELECT_TAP_MS),
        vec![GbaUp(Select)]
    );
    assert!(g.tick(5_000).is_empty());
}

#[test]
fn both_volume_keys_together_mute_once() {
    let mut g = Gestures::new();
    g.feed(Down(VolUp), 0);
    let out = g.feed(Down(VolDown), 80);
    assert!(out.contains(&MuteToggle), "the pair did not mute");
    assert!(
        g.tick(400).is_empty(),
        "it kept firing while both were held"
    );
}

#[test]
fn volume_keys_far_apart_are_not_a_chord() {
    let mut g = Gestures::new();
    g.feed(Down(VolUp), 0);
    let out = g.feed(Down(VolDown), 400);
    assert!(!out.contains(&MuteToggle), "two separate presses muted");
}

#[test]
fn the_chord_arrives_behind_the_press_that_completed_it() {
    let mut g = Gestures::new();
    g.feed(Down(VolUp), 0);
    assert_eq!(g.feed(Down(VolDown), 80), vec![VolumeDown, MuteToggle]);
}

#[test]
fn releasing_both_rearms_the_chord() {
    let mut g = Gestures::new();
    g.feed(Down(VolUp), 0);
    g.feed(Down(VolDown), 80);
    g.feed(Up(VolUp), 200);
    g.feed(Up(VolDown), 220);
    g.feed(Down(VolUp), 1_000);
    assert!(g.feed(Down(VolDown), 1_050).contains(&MuteToggle));
}

#[test]
fn a_held_volume_key_repeats() {
    let mut g = Gestures::new();
    assert_eq!(g.feed(Down(VolUp), 0), vec![VolumeUp]);
    assert!(
        g.tick(VOLUME_REPEAT_DELAY_MS - 1).is_empty(),
        "it repeated before the ramp was due"
    );
    assert_eq!(g.tick(VOLUME_REPEAT_DELAY_MS), vec![VolumeUp]);
    assert!(g.tick(VOLUME_REPEAT_DELAY_MS + 1).is_empty());
    assert_eq!(
        g.tick(VOLUME_REPEAT_DELAY_MS + VOLUME_REPEAT_MS),
        vec![VolumeUp]
    );
}

#[test]
fn a_released_volume_key_stops_repeating() {
    let mut g = Gestures::new();
    g.feed(Down(VolDown), 0);
    assert_eq!(g.tick(VOLUME_REPEAT_DELAY_MS), vec![VolumeDown]);
    assert!(g.feed(Up(VolDown), VOLUME_REPEAT_DELAY_MS + 10).is_empty());
    assert!(
        g.tick(VOLUME_REPEAT_DELAY_MS * 4).is_empty(),
        "a key nobody is holding kept ramping"
    );
}

#[test]
fn the_mute_chord_does_not_ramp() {
    let mut g = Gestures::new();
    g.feed(Down(VolUp), 0);
    assert!(g.feed(Down(VolDown), 50).contains(&MuteToggle));
    assert!(
        g.tick(VOLUME_REPEAT_DELAY_MS * 3).is_empty(),
        "the mute chord ramped the volume while it was held"
    );
}

#[test]
fn each_press_starts_its_own_ramp() {
    let mut g = Gestures::new();
    g.feed(Down(VolUp), 0);
    g.tick(VOLUME_REPEAT_DELAY_MS);
    g.feed(Up(VolUp), VOLUME_REPEAT_DELAY_MS + 5);
    assert_eq!(g.feed(Down(VolUp), 5_000), vec![VolumeUp]);
    assert!(
        g.tick(5_000 + VOLUME_REPEAT_DELAY_MS - 1).is_empty(),
        "the new press repeated early"
    );
    assert_eq!(g.tick(5_000 + VOLUME_REPEAT_DELAY_MS), vec![VolumeUp]);
}

#[test]
fn x_without_select_is_still_the_games_x() {
    let mut g = Gestures::new();
    assert_eq!(g.feed(Down(X), 0), vec![GbaDown(Btn::X)]);
    assert_eq!(g.feed(Up(X), 40), vec![GbaUp(Btn::X)]);
}

#[test]
fn x_under_select_reaches_the_game() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Select), 0).is_empty());
    assert_eq!(g.feed(Down(X), 10), vec![GbaDown(Select), GbaDown(X)]);
}

#[test]
fn select_and_either_volume_key_mutes_for_a_volume_wheel_with_no_chord() {
    for vol in [VolUp, VolDown] {
        let mut g = Gestures::new();
        assert!(g.feed(Down(Select), 0).is_empty());
        assert_eq!(g.feed(Down(vol), 100), vec![MuteToggle], "{vol:?}");
        assert!(
            g.tick(100 + VOLUME_REPEAT_DELAY_MS * 3).is_empty(),
            "{vol:?} ramped the volume under SELECT"
        );
        assert!(g.feed(Up(vol), 900).is_empty(), "{vol:?}");
        assert!(
            g.feed(Up(Select), 950).is_empty(),
            "the mute chord's SELECT reached the game"
        );
    }
}

#[test]
fn a_chord_keeps_both_keys_from_the_shelf() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Select), 0).is_empty());
    assert_eq!(g.feed(Down(Btn::Down), 50), vec![BrightnessDown]);
    assert!(
        g.feed(Up(Btn::Down), 60).is_empty(),
        "the chord key reached the shelf"
    );
    assert!(
        g.feed(Up(Select), 70).is_empty(),
        "the chord's SELECT reached the shelf"
    );
}

#[test]
fn select_chords_map_to_brightness_and_the_other_keys_are_the_shelfs() {
    for (btn, want) in [(Btn::Up, BrightnessUp), (Btn::Down, BrightnessDown)] {
        let mut g = Gestures::new();
        g.feed(RawEvent::Down(Select), 0);
        assert_eq!(g.feed(RawEvent::Down(btn), 10), vec![want]);
    }
    for btn in [Left, Right, A, B, Y, L1, R1, L2, R2] {
        let mut g = Gestures::new();
        g.feed(RawEvent::Down(Select), 0);
        assert_eq!(
            g.feed(RawEvent::Down(btn), 10),
            vec![GbaDown(Select), GbaDown(btn)],
            "{btn:?} under SELECT"
        );
    }
}

#[test]
fn a_chord_landing_late_is_still_a_chord() {
    let mut g = Gestures::new();
    g.feed(Down(Select), 0);
    g.tick(400);
    assert_eq!(g.feed(Down(Btn::Up), 400), vec![BrightnessUp]);
    assert!(
        g.tick(5_000).is_empty(),
        "the tick handed something over behind a late chord"
    );
}

#[test]
fn menu_opens_the_settings_on_the_press() {
    let mut g = Gestures::new();
    assert_eq!(g.feed(Down(Menu), 0), vec![QuickMenu]);
    assert!(g.feed(Up(Menu), 100).is_empty());
    assert!(g.tick(5_000).is_empty(), "the menu fired again on a timer");
}

#[test]
fn a_menu_held_down_opens_the_settings_once() {
    let mut g = Gestures::new();
    assert_eq!(g.feed(Down(Menu), 0), vec![QuickMenu]);
    assert!(g.tick(3_000).is_empty());
    assert!(g.feed(Up(Menu), 3_050).is_empty());
}

#[test]
fn the_power_button_does_nothing_until_it_is_held() {
    let mut g = Gestures::new();
    assert!(g.feed(Down(Btn::Power), 0).is_empty());
    assert!(g.tick(POWER_HOLD_MS - 1).is_empty());
    assert!(
        g.feed(Up(Btn::Power), POWER_HOLD_MS - 1).is_empty(),
        "a tap on power did something"
    );
    assert!(g.tick(5_000).is_empty());
}

#[test]
fn power_held_past_the_threshold_raises_the_menu_and_the_release_does_nothing() {
    let mut g = Gestures::new();
    g.feed(Down(Btn::Power), 0);
    assert!(g.tick(POWER_HOLD_MS - 1).is_empty());
    assert_eq!(g.tick(POWER_HOLD_MS), vec![PowerHold]);
    assert!(g.tick(4000).is_empty(), "the hold fires once, not per tick");
    assert!(g.feed(Up(Btn::Power), 4500).is_empty());
}

#[test]
fn a_hold_begun_again_after_a_release_fires_again() {
    let mut g = Gestures::new();
    g.feed(Down(Btn::Power), 0);
    assert_eq!(g.tick(POWER_HOLD_MS), vec![PowerHold]);
    g.feed(Up(Btn::Power), POWER_HOLD_MS + 10);
    g.feed(Down(Btn::Power), 5_000);
    assert_eq!(g.tick(5_000 + POWER_HOLD_MS), vec![PowerHold]);
}

#[test]
fn the_shoulder_triggers_are_the_shelfs_like_any_other_key() {
    let mut g = Gestures::new();
    assert_eq!(g.feed(Down(L2), 0), vec![GbaDown(L2)]);
    assert_eq!(g.feed(Up(L2), 40), vec![GbaUp(L2)]);
    assert_eq!(g.feed(Down(R2), 60), vec![GbaDown(R2)]);
    assert_eq!(g.feed(Up(R2), 90), vec![GbaUp(R2)]);
}
