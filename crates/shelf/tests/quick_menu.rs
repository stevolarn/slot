mod common;

use common::{
    app_booting_at, app_booting_with_clock, insert, tmp_root_with_carts, Clock, CLOCK_IS_SET,
};
use shelf::app::{App, Phase};
use shelf_input::{Action, Btn};
use shelf_store::{read_slot_state, write_slot_state, SlotState};
use shelf_ui::{
    edge, Draw, Icon, QuickMenuFaces, QuickRow, QuickValue, TexId, MENU_PAD, OUT_W, QUICK_EDGE,
    QUICK_PITCH, QUICK_TOP,
};
use tempfile::TempDir;

fn on_carousel_with(state: SlotState) -> (TempDir, App, Clock) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..state
        },
    )
    .expect("write slot.state");
    let (a, clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "not on the carousel: {:?}",
        a.phase()
    );
    (d, a, clock)
}

fn on_carousel() -> (TempDir, App, Clock) {
    on_carousel_with(SlotState::default())
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn open_at(a: &mut App, row: QuickRow) {
    a.apply(Action::QuickMenu);
    for _ in 0..row.index() {
        press(a, Btn::Down);
    }
    assert_eq!(a.quick_menu(), Some(row), "the bar never reached {row:?}");
}

#[test]
fn menu_opens_the_settings_with_its_top_row_selected_every_time() {
    let (_d, mut a, _) = on_carousel();
    a.apply(Action::QuickMenu);
    assert_eq!(a.quick_menu(), Some(QuickRow::ALL[0]));
    press(&mut a, Btn::Down);
    a.apply(Action::QuickMenu);
    a.apply(Action::QuickMenu);
    assert_eq!(
        a.quick_menu(),
        Some(QuickRow::ALL[0]),
        "the menu opened where it was last left"
    );
}

#[test]
fn the_settings_are_crt_mode_the_clock_and_about() {
    let labels: Vec<&str> = QuickRow::ALL.iter().map(|r| r.label()).collect();
    assert_eq!(labels, ["CRT Mode", "Date & Time", "About"]);
}

#[test]
fn menu_or_b_closes_it_back_onto_the_carousel_where_you_were() {
    for close in [Action::QuickMenu, Action::GbaDown(Btn::B)] {
        let (_d, mut a, _) = on_carousel();
        press(&mut a, Btn::Right);
        assert_eq!(
            a.selected_stem(),
            Some("Fusion"),
            "the carousel never moved"
        );
        a.apply(Action::QuickMenu);
        a.apply(close);
        assert!(
            matches!(a.phase(), Phase::Shelf),
            "{close:?} left {:?}",
            a.phase()
        );
        assert_eq!(a.selected_stem(), Some("Fusion"), "{close:?} lost the cart");
    }
}

#[test]
fn the_quick_menu_is_only_on_the_shelf() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = common::boot(d.path());
    insert(&mut a);
    a.apply(Action::QuickMenu);
    assert!(
        matches!(a.phase(), Phase::Inserting { .. }),
        "MENU raised the quick menu over a cart going in: {:?}",
        a.phase()
    );
    assert_eq!(a.quick_menu(), None);
}

#[test]
fn up_and_down_move_the_bar_and_wrap_at_the_ends() {
    let (_d, mut a, _) = on_carousel();
    a.apply(Action::QuickMenu);
    press(&mut a, Btn::Up);
    assert_eq!(
        a.quick_menu(),
        Some(QuickRow::About),
        "up from the top did not wrap to the bottom"
    );
    for want in [
        QuickRow::Crt,
        QuickRow::DateTime,
        QuickRow::About,
        QuickRow::Crt,
    ] {
        press(&mut a, Btn::Down);
        assert_eq!(a.quick_menu(), Some(want));
    }
}

#[test]
fn crt_mode_flips_on_either_arrow_and_saves() {
    let (d, mut a, _) = on_carousel();
    open_at(&mut a, QuickRow::Crt);
    assert_eq!(a.quick_value(QuickRow::Crt), Some(QuickValue::Off));
    for (btn, want) in [
        (Btn::Right, true),
        (Btn::Left, false),
        (Btn::Left, true),
        (Btn::Right, false),
    ] {
        press(&mut a, btn);
        assert_eq!(
            read_slot_state(d.path()).crt,
            want,
            "{btn:?} never reached the card"
        );
        assert_eq!(a.quick_value(QuickRow::Crt), Some(QuickValue::flag(want)));
    }
}

#[test]
fn crt_mode_leaves_the_levels_around_it_alone() {
    let (d, mut a, _) = on_carousel();
    open_at(&mut a, QuickRow::Crt);
    press(&mut a, Btn::Right);
    let s = read_slot_state(d.path());
    assert!(s.crt, "the row never took");
    let fresh = SlotState::default();
    assert_eq!(
        (s.brightness, s.volume, s.muted),
        (fresh.brightness, fresh.volume, fresh.muted),
        "the row reached a setting that is not its own"
    );
}

#[test]
fn the_crt_overlay_follows_the_setting() {
    let (_d, mut a, _) = on_carousel();
    let lit = |a: &App| {
        let mut out = Vec::new();
        a.draw(&mut out);
        out.len()
    };
    let before = lit(&a);
    open_at(&mut a, QuickRow::Crt);
    press(&mut a, Btn::Right);
    press(&mut a, Btn::B);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert!(
        lit(&a) > before,
        "the scanlines were not drawn over the shelf"
    );
}

#[test]
fn the_arrows_change_nothing_on_a_row_that_opens() {
    let (d, mut a, _) = on_carousel();
    let before = std::fs::read(d.path().join("Config/slot.state")).expect("read slot.state");
    for row in [QuickRow::DateTime, QuickRow::About] {
        open_at(&mut a, row);
        press(&mut a, Btn::Left);
        press(&mut a, Btn::Right);
        assert_eq!(a.quick_menu(), Some(row), "an arrow left {row:?}");
        a.apply(Action::QuickMenu);
    }
    assert_eq!(
        std::fs::read(d.path().join("Config/slot.state")).expect("read slot.state"),
        before,
        "an arrow on a row that opens wrote the card"
    );
}

#[test]
fn a_on_about_opens_the_label_and_b_comes_back_to_the_menu() {
    let (_d, mut a, _) = on_carousel();
    open_at(&mut a, QuickRow::About);
    press(&mut a, Btn::A);
    assert!(matches!(a.phase(), Phase::About), "{:?}", a.phase());
    press(&mut a, Btn::B);
    assert_eq!(a.quick_menu(), Some(QuickRow::About));
}

#[test]
fn a_on_date_and_time_opens_the_clock_at_the_time_it_already_has() {
    let (_d, mut a, _) = on_carousel_with(SlotState {
        utc_offset_min: -300,
        ..SlotState::default()
    });
    open_at(&mut a, QuickRow::DateTime);
    press(&mut a, Btn::A);
    let picker = a.picker().expect("A on Date & Time did not open the clock");
    assert_eq!(picker.offset_min(), -300, "the offset started over");
    assert_eq!(
        picker.secs(),
        CLOCK_IS_SET - CLOCK_IS_SET % 60,
        "the picker does not show the minute the clock is in"
    );
}

#[test]
fn confirming_the_clock_from_the_menu_sets_it_and_comes_back_to_the_menu() {
    let (d, mut a, clock) = on_carousel_with(SlotState {
        utc_offset_min: -300,
        ..SlotState::default()
    });
    open_at(&mut a, QuickRow::DateTime);
    press(&mut a, Btn::A);
    press(&mut a, Btn::Up);
    for _ in 0..5 {
        press(&mut a, Btn::Right);
    }
    press(&mut a, Btn::Down);
    let changed = a.picker().expect("not on the clock").secs() - (CLOCK_IS_SET - CLOCK_IS_SET % 60);
    press(&mut a, Btn::A);
    assert_eq!(a.quick_menu(), Some(QuickRow::DateTime), "{:?}", a.phase());
    assert_eq!(
        clock.get(),
        CLOCK_IS_SET + changed,
        "the platform clock was not set"
    );
    let s = read_slot_state(d.path());
    assert_eq!(s.utc_offset_min, -330, "the offset was not saved");
    assert!(s.clock_set);
}

#[test]
fn b_on_the_clock_from_the_menu_comes_back_without_changing_anything() {
    let (d, mut a, clock) = on_carousel_with(SlotState {
        utc_offset_min: -300,
        ..SlotState::default()
    });
    open_at(&mut a, QuickRow::DateTime);
    press(&mut a, Btn::A);
    press(&mut a, Btn::Up);
    for _ in 0..5 {
        press(&mut a, Btn::Right);
    }
    press(&mut a, Btn::Down);
    press(&mut a, Btn::B);
    assert_eq!(a.quick_menu(), Some(QuickRow::DateTime), "{:?}", a.phase());
    assert_eq!(clock.get(), CLOCK_IS_SET, "B set the clock");
    assert_eq!(
        read_slot_state(d.path()).utc_offset_min,
        -300,
        "B saved the offset"
    );
}

#[test]
fn the_first_boot_clock_still_has_no_way_back() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let (mut a, _) = app_booting_with_clock(d.path());
    press(&mut a, Btn::B);
    a.apply(Action::QuickMenu);
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "left the first boot clock: {:?}",
        a.phase()
    );
}

#[test]
fn brightness_and_volume_still_answer_over_the_quick_menu() {
    let (d, mut a, _) = on_carousel();
    let icons: Vec<TexId> = (0..Icon::ALL.len())
        .map(|i| TexId::from_raw(700 + i))
        .collect();
    a.set_icon_faces(icons.clone());
    open_at(&mut a, QuickRow::Crt);
    let before = read_slot_state(d.path());
    a.apply(Action::BrightnessUp);
    a.apply(Action::VolumeDown);
    a.flush_state();
    let after = read_slot_state(d.path());
    assert_eq!(
        (after.brightness, after.volume),
        (before.brightness + 1, before.volume - 5)
    );
    assert_eq!(
        a.quick_menu(),
        Some(QuickRow::Crt),
        "a level moved the menu"
    );
    let out = frame(&a);
    assert!(
        out.iter()
            .any(|d| matches!(*d, Draw::Tex { tex, .. } if icons.contains(&tex))),
        "the level's bar is not drawn over the menu"
    );
}

fn fake_faces(a: &mut App) {
    let id = TexId::from_raw;
    a.set_quick_menu_faces(QuickMenuFaces {
        labels: (0..QuickRow::ALL.len())
            .map(|i| (id(100 + i), 200, 40))
            .collect(),
        values: (0..QuickValue::ALL.len())
            .map(|i| [(id(200 + i), 60, 40), (id(210 + i), 60, 40)])
            .collect(),
        carets: [(id(300), 10, 40), (id(301), 10, 40)],
        legend: [(id(400), 70), (id(401), 110), (id(402), 80)],
    });
    a.set_quick_clock_faces((id(500), 150, 40), (id(501), 150, 40));
}

fn value(v: QuickValue, lit: bool) -> usize {
    if lit {
        210 + v.index()
    } else {
        200 + v.index()
    }
}

fn frame(a: &App) -> Vec<Draw> {
    let mut out = Vec::new();
    a.draw(&mut out);
    out
}

fn placed(out: &[Draw], id: usize) -> Option<[f32; 4]> {
    out.iter().find_map(|d| match *d {
        Draw::Tex {
            x, y, w, h, tex, ..
        } if tex == TexId::from_raw(id) => Some([x, y, w, h]),
        _ => None,
    })
}

fn drawn(out: &[Draw], id: usize) -> bool {
    placed(out, id).is_some()
}

#[test]
fn the_legend_says_change_on_a_value_row_and_open_on_a_row_that_opens() {
    let (_d, mut a, _) = on_carousel();
    fake_faces(&mut a);
    a.apply(Action::QuickMenu);
    for row in QuickRow::ALL {
        assert_eq!(a.quick_menu(), Some(row));
        let out = frame(&a);
        assert!(drawn(&out, 400), "no B BACK on {row:?}");
        assert_eq!(drawn(&out, 401), !row.opens(), "CHANGE on {row:?}");
        assert_eq!(drawn(&out, 402), row.opens(), "OPEN on {row:?}");
        press(&mut a, Btn::Down);
    }
}

#[test]
fn the_arrows_stand_only_around_the_selected_rows_value() {
    let (_d, mut a, _) = on_carousel();
    fake_faces(&mut a);
    open_at(&mut a, QuickRow::Crt);
    let out = frame(&a);
    assert!(
        drawn(&out, 300) && drawn(&out, 301),
        "no arrows on CRT Mode"
    );
    assert!(
        drawn(&out, value(QuickValue::Off, true)),
        "CRT's Off is not lit"
    );

    press(&mut a, Btn::Down);
    let out = frame(&a);
    assert!(
        !drawn(&out, 300) && !drawn(&out, 301),
        "arrows on a row that opens"
    );
    assert!(drawn(&out, 501), "the date and time in hand is not lit");
    assert!(
        drawn(&out, value(QuickValue::Off, false)),
        "CRT's Off is not grey"
    );
}

#[test]
fn the_bar_runs_edge_to_edge_behind_the_selected_row() {
    let (_d, mut a, _) = on_carousel();
    fake_faces(&mut a);
    a.apply(Action::QuickMenu);
    for row in QuickRow::ALL {
        let bars: Vec<_> = frame(&a)
            .into_iter()
            .filter_map(|d| match d {
                Draw::Rect { x, y, w, h, colour } if colour == edge() => Some([x, y, w, h]),
                _ => None,
            })
            .collect();
        let top = QUICK_TOP + QUICK_PITCH * row.index() as f32;
        assert_eq!(
            bars,
            vec![[0.0, top + 4.0, OUT_W as f32, QUICK_PITCH - 8.0]],
            "{row:?}"
        );
        press(&mut a, Btn::Down);
    }
}

#[test]
fn labels_start_and_values_end_thirty_two_pixels_in() {
    let (_d, mut a, _) = on_carousel();
    fake_faces(&mut a);
    open_at(&mut a, QuickRow::DateTime);
    let out = frame(&a);
    let right = OUT_W as f32 - QUICK_EDGE;
    for row in QuickRow::ALL {
        let [x, ..] = placed(&out, 100 + row.index()).expect("a label was not drawn");
        assert_eq!(x + MENU_PAD as f32, QUICK_EDGE, "{row:?}'s label");
    }
    let [x, _, w, _] = placed(&out, value(QuickValue::Off, false)).expect("CRT's value");
    assert_eq!(x + w - MENU_PAD as f32, right, "an unselected value");
    press(&mut a, Btn::Up);
    let out = frame(&a);
    let [x, _, w, _] = placed(&out, 301).expect("the right arrow");
    assert_eq!(x + w, right, "the arrow around the selected value");
}

#[test]
fn only_the_clock_from_the_menu_offers_b_back() {
    let (_d, mut a, _) = on_carousel();
    fake_faces(&mut a);
    a.set_clock_faces(TexId::from_raw(600), TexId::from_raw(601));
    open_at(&mut a, QuickRow::DateTime);
    press(&mut a, Btn::A);
    let out = frame(&a);
    assert!(drawn(&out, 601), "the clock lost its own key");
    assert!(
        drawn(&out, 400),
        "the clock from the menu does not offer B BACK"
    );

    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let (mut first, _) = app_booting_with_clock(d.path());
    fake_faces(&mut first);
    first.set_clock_faces(TexId::from_raw(600), TexId::from_raw(601));
    let out = frame(&first);
    assert!(drawn(&out, 601), "the first boot clock lost its key");
    assert!(
        !drawn(&out, 400),
        "the first boot clock offers a way back it does not have"
    );
}
