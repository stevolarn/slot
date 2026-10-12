mod common;

use common::{app_booting_at, app_booting_with_clock, tmp_root_with_carts, Clock};
use shelf::app::{App, Phase};
use shelf_input::{Action, Btn};
use shelf_store::{read_slot_state, write_slot_state, SlotState};
use shelf_ui::{ClockPicker, Field, QuickRow};
use tempfile::TempDir;

#[test]
fn the_clock_is_asked_for_once_and_only_once() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut a, _) = app_booting_with_clock(d.path());
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "not asked with the platform clock unset"
    );

    a.confirm_clock();
    assert!(read_slot_state(d.path()).clock_set);
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "confirming did not go on to the shelf: {:?}",
        a.phase()
    );
    let (again, _) = app_booting_at(d.path(), 1_786_568_000);
    assert!(
        !matches!(again.phase(), Phase::SetClock { .. }),
        "asked twice"
    );
}

#[test]
fn a_platform_clock_that_is_set_is_never_asked_about_at_boot() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (a, _) = app_booting_at(d.path(), 1_786_568_000);
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "asked for a clock that was already right: {:?}",
        a.phase()
    );
}

#[test]
fn the_clock_comes_before_the_shelf_goes_back_to_the_cart_that_was_played() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            cart: Some("Fusion".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let (mut a, _) = app_booting_with_clock(d.path());
    assert!(matches!(a.phase(), Phase::SetClock { .. }));
    a.confirm_clock();
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(a.selected_stem(), Some("Fusion"));
}

#[test]
fn confirming_writes_the_picked_time_to_the_platform() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut a, clock) = app_booting_with_clock(d.path());
    a.apply(Action::GbaDown(Btn::Up));
    a.confirm_clock();
    assert_ne!(clock.get(), 0, "the platform clock was never set");
}

#[test]
fn the_picker_starts_from_the_clock_the_platform_already_has() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut a, clock) = app_booting_at(d.path(), 1_700_000_000);
    a.confirm_clock();
    assert_eq!(
        clock.get(),
        1_700_000_000,
        "confirming the clock as it stands moved it"
    );
}

#[test]
fn a_field_wraps_rather_than_running_off_the_end() {
    let mut c = ClockPicker::from_secs(0);
    c.field(Field::Month);
    for _ in 0..13 {
        c.up();
    }
    assert!(
        (1..=12).contains(&c.month()),
        "month {} left the calendar",
        c.month()
    );
}

#[test]
fn february_29_is_reachable_in_a_leap_year_and_not_otherwise() {
    let mut c = ClockPicker::from_ymd(2028, 2, 28);
    c.field(Field::Day);
    c.up();
    assert_eq!(c.day(), 29, "2028 is a leap year");
    let mut c = ClockPicker::from_ymd(2027, 2, 28);
    c.field(Field::Day);
    c.up();
    assert_eq!(c.day(), 1, "2027 has no 29th of February");
}

#[test]
fn a_day_the_new_month_does_not_have_is_pulled_back() {
    let mut c = ClockPicker::from_ymd(2026, 1, 31);
    c.field(Field::Month);
    c.up();
    assert_eq!((c.month(), c.day()), (2, 28));
}

#[test]
fn the_picker_hands_back_utc_rather_than_what_was_typed() {
    let mut c = ClockPicker::from_ymd(2026, 8, 12);
    c.field(Field::Hour);
    for _ in 0..9 {
        c.up();
    }
    let typed = c.secs();
    c.field(Field::Offset);
    for _ in 0..10 {
        c.down();
    }
    assert_eq!(c.offset_min(), -300);
    assert_eq!(
        c.secs(),
        typed + 300 * 60,
        "09:00 local at UTC-5 is 14:00 UTC"
    );
}

#[test]
fn the_offset_steps_in_half_hours_because_real_zones_do() {
    let mut c = ClockPicker::from_ymd(2026, 8, 12);
    c.field(Field::Offset);
    c.up();
    assert_eq!(c.offset_min(), 30);
    c.up();
    assert_eq!(c.offset_min(), 60);
}

#[test]
fn the_offset_stops_at_the_ends_of_the_real_range() {
    let mut c = ClockPicker::from_ymd(2026, 8, 12);
    c.field(Field::Offset);
    for _ in 0..40 {
        c.up();
    }
    assert_eq!(c.offset_min(), 840, "walked past UTC+14");
    for _ in 0..80 {
        c.down();
    }
    assert_eq!(c.offset_min(), -720, "walked past UTC-12");
}

#[test]
fn the_offset_is_part_of_the_line_of_type() {
    let mut c = ClockPicker::from_ymd(2026, 8, 12);
    let before = c.text();
    c.field(Field::Offset);
    c.down();
    assert_ne!(c.text(), before);
    assert!(c.text().contains("-00:30"), "{}", c.text());
}

#[test]
fn confirming_persists_the_offset_and_sets_the_platform_to_utc() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut a, clock) = app_booting_at(d.path(), 1_500_000_000);
    for _ in 0..8 {
        a.apply(Action::GbaDown(Btn::Right));
    }
    for _ in 0..4 {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.confirm_clock();
    assert_eq!(read_slot_state(d.path()).utc_offset_min, -120);
    assert_eq!(
        clock.get(),
        1_500_000_000 + 120 * 60,
        "the platform was set to local rather than to utc"
    );
}

#[test]
fn the_wall_clock_is_local_rather_than_the_utc_the_card_keeps() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            utc_offset_min: -300,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 1_700_000_000);
    assert_eq!(a.wall_secs(), 1_700_000_000 - 300 * 60);
}

#[test]
fn a_clock_that_never_got_set_is_asked_for_again() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 0);
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "a 1970 clock was taken at face value: {:?}",
        a.phase()
    );
}

#[test]
fn a_clock_that_looks_like_a_real_date_is_not_asked_for_again() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 1_786_568_000);
    assert!(
        !matches!(a.phase(), Phase::SetClock { .. }),
        "asked again for a clock that was already right"
    );
}

/// A platform clock that has been set, and one before 2020 that has not.
fn at(way: Way) -> i64 {
    match way {
        Way::FirstBoot => 1_500_000_022,
        Way::QuickMenu => 1_786_568_022,
    }
}
const OPEN_FOR: i64 = 100;

#[derive(Copy, Clone, Debug)]
enum Way {
    FirstBoot,
    QuickMenu,
}

fn clock_screen(way: Way) -> (TempDir, App, Clock) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    if let Way::QuickMenu = way {
        write_slot_state(
            d.path(),
            &SlotState {
                clock_set: true,
                utc_offset_min: -300,
                ..SlotState::default()
            },
        )
        .unwrap();
    }
    let (mut a, clock) = app_booting_at(d.path(), at(way));
    if let Way::QuickMenu = way {
        a.apply(Action::QuickMenu);
        for _ in 0..QuickRow::DateTime.index() {
            a.apply(Action::GbaDown(Btn::Down));
        }
        a.apply(Action::GbaDown(Btn::A));
    }
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "{way:?} never reached the clock: {:?}",
        a.phase()
    );
    (d, a, clock)
}

#[test]
fn confirming_the_clock_untouched_leaves_it_where_it_is() {
    for way in [Way::FirstBoot, Way::QuickMenu] {
        let (_d, mut a, clock) = clock_screen(way);
        let at = at(way);
        clock.advance(OPEN_FOR);
        a.apply(Action::GbaDown(Btn::A));
        assert_eq!(
            clock.get(),
            at + OPEN_FOR,
            "{way:?}: an untouched confirm moved the clock by {} s",
            clock.get() - (at + OPEN_FOR)
        );
    }
}

#[test]
fn changing_the_minute_moves_the_clock_by_exactly_that_minute() {
    for way in [Way::FirstBoot, Way::QuickMenu] {
        let (_d, mut a, clock) = clock_screen(way);
        let at = at(way);
        for _ in 0..4 {
            a.apply(Action::GbaDown(Btn::Right));
        }
        assert_eq!(a.picker().expect("on the clock").cursor(), Field::Minute);
        a.apply(Action::GbaDown(Btn::Up));
        clock.advance(OPEN_FOR);
        a.apply(Action::GbaDown(Btn::A));
        assert_eq!(
            clock.get(),
            at + OPEN_FOR + 60,
            "{way:?}: a minute on moved the clock by {} s",
            clock.get() - (at + OPEN_FOR)
        );
    }
}

#[test]
fn changing_only_the_offset_moves_utc_by_exactly_that_change() {
    for way in [Way::FirstBoot, Way::QuickMenu] {
        let (d, mut a, clock) = clock_screen(way);
        let at = at(way);
        let before = a.picker().expect("on the clock").offset_min();
        for _ in 0..5 {
            a.apply(Action::GbaDown(Btn::Right));
        }
        a.apply(Action::GbaDown(Btn::Down));
        clock.advance(OPEN_FOR);
        a.apply(Action::GbaDown(Btn::A));
        assert_eq!(
            clock.get(),
            at + OPEN_FOR + 30 * 60,
            "{way:?}: half an hour west moved the clock by {} s",
            clock.get() - (at + OPEN_FOR)
        );
        assert_eq!(
            i64::from(read_slot_state(d.path()).utc_offset_min),
            before - 30,
            "{way:?}: the offset was not saved"
        );
    }
}
