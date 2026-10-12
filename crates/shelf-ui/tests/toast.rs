use shelf_ui::{toast_face, toast_rect, Draw, Hud, HudKind, Toast, ALARM_BAND_H, OUT_H, OUT_W};

fn band(h: &Hud, now: u64) -> Option<(f32, f32, [f32; 4])> {
    let mut out = Vec::new();
    h.draw(now, &mut out);
    out.iter().rev().find_map(|d| match d {
        Draw::Rect {
            y, w, h, colour, ..
        } if *w == OUT_W as f32 && *h == ALARM_BAND_H => Some((*y, *h, *colour)),
        _ => None,
    })
}

#[test]
fn the_card_alarm_stays_up_until_it_is_cleared() {
    let mut h = Hud::new();
    h.set_alarm(true, 1_000);
    assert!(
        band(&h, 3_600_000).is_some(),
        "the alarm went away by itself"
    );
    h.set_alarm(false, 3_600_001);
    let mut out = Vec::new();
    h.draw(3_600_002, &mut out);
    assert!(out.is_empty(), "the alarm outlived a write that landed");
}

#[test]
fn the_card_alarm_is_a_red_band_across_the_middle_that_pulses() {
    let mut h = Hud::new();
    h.set_alarm(true, 0);
    let (y, bh, bright) = band(&h, 0).expect("no alarm band");
    let middle = OUT_H as f32 / 2.0;
    assert!(
        (y + bh / 2.0 - middle).abs() < 0.5,
        "the band is not centred"
    );
    assert!(
        bright[0] > 2.0 * bright[1] && bright[0] > 2.0 * bright[2],
        "{bright:?} is not red"
    );
    let (_, _, dim) = band(&h, 250).unwrap();
    assert!(
        dim[0] < bright[0] - 0.2,
        "the band did not pulse: {bright:?} then {dim:?}"
    );
}

#[test]
fn a_second_failure_does_not_restart_the_pulse() {
    let mut h = Hud::new();
    h.set_alarm(true, 0);
    h.set_alarm(true, 250);
    let mut fresh = Hud::new();
    fresh.set_alarm(true, 0);
    assert_eq!(band(&h, 500), band(&fresh, 500));
}

#[test]
fn the_card_alarm_text_is_large_and_inside_its_band() {
    let alarm = toast_face(Toast::CardUnwritable);
    assert!(alarm.h > 0 && alarm.w > 0);
    let (x, y, w, th) = toast_rect(Toast::CardUnwritable);
    let top = (OUT_H as f32 - ALARM_BAND_H) / 2.0;
    assert!(
        y >= top && y + th <= top + ALARM_BAND_H,
        "the alarm text overhangs its band"
    );
    assert_eq!(x + w / 2.0, OUT_W as f32 / 2.0);
}

#[test]
fn the_card_alarm_is_drawn_over_the_volume_bar() {
    let mut h = Hud::new();
    h.set_alarm(true, 0);
    h.show(HudKind::Volume, 50, false, 1_000);
    let mut out = Vec::new();
    h.draw(1_100, &mut out);
    let alarm = out
        .iter()
        .position(|d| matches!(d, Draw::Rect { h, .. } if *h == ALARM_BAND_H))
        .expect("the bar hid the alarm");
    assert!(alarm > 0, "the bar was not drawn under the alarm");
    assert!(
        out[alarm + 1..]
            .iter()
            .all(|d| matches!(d, Draw::Tex { .. })),
        "something was drawn over the alarm band"
    );
}

#[test]
fn the_card_alarm_says_what_failed_and_what_it_costs() {
    assert_eq!(Toast::CardUnwritable.text(), "Can't write to SD card");
    assert_eq!(
        Toast::CardUnwritable.detail(),
        Some("Progress is not being saved")
    );
}

#[test]
fn the_card_alarm_inks_both_of_its_lines() {
    let f = toast_face(Toast::CardUnwritable);
    let inked = |rows: std::ops::Range<u32>| {
        rows.into_iter()
            .any(|y| (0..f.w).any(|x| f.rgba[((y * f.w + x) * 4 + 3) as usize] > 0x80))
    };
    let half = f.h / 2;
    assert!(inked(0..half), "the first line is blank");
    assert!(inked(half + 8..f.h), "the second line is blank");
}

#[test]
fn every_banner_answers_to_its_place_in_the_list_and_has_something_to_draw() {
    let all = Toast::all();
    assert_eq!(
        all.len(),
        8,
        "a banner was added or dropped: every face is uploaded by its place in this list"
    );
    assert_eq!(all[0], Toast::CardUnwritable);
    for (i, t) in all.iter().enumerate() {
        assert_eq!(t.index(), i, "{t:?} does not answer to its own place");
        let f = toast_face(*t);
        assert!(
            f.rgba.chunks(4).any(|p| p[3] > 0),
            "{t:?} rastered to a blank banner"
        );
    }
}

#[test]
fn the_card_unwritable_banner_is_the_only_alarm() {
    assert!(Toast::CardUnwritable.alarm());
}

#[test]
fn a_banner_fades_on_the_same_curve_as_the_bar() {
    let mut h = Hud::new();
    h.toast(Toast::CardUnwritable, 1_000);
    assert!(h.toast_visible(2_499));
    assert!(!h.toast_visible(2_500));
}

#[test]
fn saying_the_same_thing_twice_re_shows_it_rather_than_stacking() {
    let mut h = Hud::new();
    h.toast(Toast::CardUnwritable, 1_000);
    h.toast(Toast::CardUnwritable, 2_400);
    assert_eq!(
        h.said(3_400),
        Some(Toast::CardUnwritable),
        "it did not re-show"
    );
    assert_eq!(h.said(3_900), None, "it never faded");
}

#[test]
fn a_banner_carries_its_own_halo() {
    let f = toast_face(Toast::CardUnwritable);
    let dark = f
        .rgba
        .chunks(4)
        .any(|p| p[3] > 0 && p[0] < 0x40 && p[1] < 0x40 && p[2] < 0x40);
    assert!(dark, "there is nothing dark behind the type");
}
