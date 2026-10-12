use shelf_ui::{Draw, Hud, HudKind, Icon, TexId, Toast, HUD_MS, OUT_H, OUT_W};

fn bottom_edge(d: &Draw) -> Option<f32> {
    match *d {
        Draw::Rect { y, h, .. } | Draw::Tex { y, h, .. } | Draw::Turned { y, h, .. } => Some(y + h),
    }
}

fn fill(kind: HudKind, value: u8) -> f32 {
    let mut h = Hud::new();
    h.show(kind, value, false, 0);
    let mut out = Vec::new();
    h.draw(0, &mut out);
    match out.last().expect("no bar in the list") {
        Draw::Rect { w, .. } => *w,
        _ => panic!("the bar must be a rect"),
    }
}

#[test]
fn hud_fades_after_1500ms() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 5, false, 1000);
    assert!(h.visible(2499));
    assert!(!h.visible(2500));
}

#[test]
fn nothing_shows_until_something_is_adjusted() {
    assert!(!Hud::new().visible(0));
}

#[test]
fn volume_reads_against_100_not_the_nine_step_scale() {
    let full = fill(HudKind::Brightness, 9);
    assert!(
        fill(HudKind::Volume, 9) < full * 0.2,
        "volume 9 filled the bar, so it is being read against the brightness scale"
    );
    assert_eq!(fill(HudKind::Volume, 100), full);
}

#[test]
fn the_hud_sits_at_the_top_of_the_screen() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    let mut out = Vec::new();
    h.draw(0, &mut out);
    let lowest = out.iter().filter_map(bottom_edge).fold(0.0f32, f32::max);
    assert!(
        lowest < OUT_H as f32 / 2.0,
        "hud reaches {lowest}, expected the top half"
    );
}

#[test]
fn the_hud_draws_a_dark_plate_behind_itself() {
    let mut h = Hud::new();
    h.show(HudKind::Brightness, 5, false, 0);
    let mut out = Vec::new();
    h.draw(0, &mut out);
    let plate = out.first().expect("nothing drawn");
    let Draw::Rect { w, colour, .. } = plate else {
        panic!("first draw is not the plate")
    };
    assert_eq!(*w, OUT_W as f32, "the plate does not span the screen");
    assert!(
        colour[0] < 0.2 && colour[1] < 0.2 && colour[2] < 0.2,
        "the plate is not dark"
    );
    assert!(
        colour[3] > 0.6,
        "the plate is too transparent to give contrast"
    );
}

#[test]
fn muted_volume_uses_the_muted_icon() {
    assert_eq!(HudKind::Volume.icon(0, false), Icon::VolumeZero);
    assert_eq!(HudKind::Volume.icon(0, true), Icon::VolumeMuted);
    assert_eq!(HudKind::Volume.icon(40, true), Icon::VolumeMuted);
    assert_eq!(HudKind::Volume.icon(40, false), Icon::Volume);
}

#[test]
fn brightness_keeps_its_icon_whatever_the_state() {
    assert_eq!(HudKind::Brightness.icon(4, false), Icon::Brightness);
    assert_eq!(HudKind::Brightness.icon(0, true), Icon::Brightness);
}

#[test]
fn a_toast_replaces_the_bar_on_the_plate_and_goes_away_on_its_own() {
    let mut h = Hud::new();
    let toasts: Vec<TexId> = Toast::all()
        .iter()
        .enumerate()
        .map(|(i, _)| TexId::from_raw(800 + i))
        .collect();
    h.set_toasts(toasts.clone());
    h.show(HudKind::Volume, 50, false, 1_000);
    h.toast(Toast::CardUnwritable, 1_000);
    assert_eq!(h.said(1_000), Some(Toast::CardUnwritable));
    let mut out = Vec::new();
    h.draw(1_000, &mut out);
    assert!(
        out.iter().any(
            |d| matches!(d, Draw::Tex { tex, .. } if *tex == toasts[Toast::CardUnwritable.index()])
        ),
        "the toast was not drawn"
    );
    assert_eq!(h.said(1_000 + HUD_MS), None, "the toast never went away");
}

#[test]
fn the_card_alarm_pulses_until_it_is_cleared() {
    let mut h = Hud::new();
    assert!(!h.alarmed());
    h.set_alarm(true, 0);
    assert!(h.alarmed());
    let mut out = Vec::new();
    h.draw(10_000, &mut out);
    assert!(
        out.iter()
            .any(|d| matches!(d, Draw::Rect { w, .. } if *w == OUT_W as f32)),
        "the alarm band is missing"
    );
    h.set_alarm(false, 10_000);
    let mut out = Vec::new();
    h.draw(10_001, &mut out);
    assert!(out.is_empty(), "the alarm outlived the fault");
}
