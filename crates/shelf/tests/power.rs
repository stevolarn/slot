mod common;

use std::sync::atomic::Ordering;

use common::{boot, panel, session_with_platform, tmp_root_with_carts};
use shelf::app::Phase;
use shelf::session::Session;
use shelf_gfx::Draw;
use shelf_input::{Action, Btn, Millis, RawEvent, POWER_HOLD_MS};
use shelf_store::{write_shelf_state, ShelfState};
use shelf_ui::QuickRow;

const FRAME_MS: Millis = 16;
const DT: f32 = 1.0 / 60.0;

#[test]
fn the_backlight_follows_brightness_from_boot() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_shelf_state(
        d.path(),
        &ShelfState {
            brightness: 3,
            clock_set: true,
            utc_offset_min: 0,
            ..Default::default()
        },
    )
    .unwrap();
    let mut a = boot(d.path());
    let (power, step) = panel(d.path());
    a.set_power(power);
    assert_eq!(step.load(Ordering::Relaxed), 3);
    a.apply(Action::BrightnessUp);
    assert_eq!(step.load(Ordering::Relaxed), 4);
}

#[test]
fn the_menu_covers_the_screen_in_the_case_materials() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::PowerHold);

    let mut out = Vec::new();
    a.draw(&mut out);

    match out.first() {
        Some(Draw::Rect { w, h, colour, .. }) => {
            assert_eq!(
                *colour,
                shelf_ui::opening(),
                "the ground is the case's opening"
            );
            assert!(*w > 0.0 && *h > 0.0, "and it covers the panel");
        }
        other => panic!("the menu drew {other:?} rather than a ground"),
    }
    assert!(
        matches!(out.get(1), Some(Draw::Rect { colour, .. }) if *colour == shelf_ui::edge()),
        "the bar of the selected row follows the ground"
    );
    assert_eq!(
        out.len(),
        2,
        "nothing of the previous phase survives the menu"
    );
}

#[test]
fn a_power_off_draws_a_shutdown_screen_over_everything() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::PowerHold);
    a.apply(Action::GbaDown(Btn::A));

    let mut out = Vec::new();
    a.draw(&mut out);

    match out.first() {
        Some(Draw::Rect { w, h, colour, .. }) => {
            assert_eq!(*colour, [0.0, 0.0, 0.0, 1.0], "the shutdown is black");
            assert!(*w > 0.0 && *h > 0.0, "and covers the panel");
        }
        other => panic!("the shutdown drew {other:?} rather than a panel of black"),
    }
    assert_eq!(
        out.len(),
        1,
        "nothing of the previous phase survives the shutdown screen"
    );
}

#[test]
fn a_hold_opens_the_menu_on_power_off_and_commits_nothing() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::PowerHold);
    assert_eq!(
        a.quick_menu(),
        Some(QuickRow::PowerOff),
        "the hold did not select Power Off"
    );
    assert!(!a.powering_off() && !a.restarting() && !a.exiting());
}

#[test]
fn the_menu_has_options_about_restart_power_off_and_exit() {
    let rows: Vec<&str> = QuickRow::TOP.iter().map(|r| r.label()).collect();
    assert_eq!(rows, ["Options", "Button Guide", "About", "Restart", "Power Off", "Exit"]);
}

#[test]
fn the_menu_raised_by_a_hold_moves_and_wraps_at_both_ends() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::PowerHold);
    a.apply(Action::GbaDown(Btn::Down));
    assert_eq!(a.quick_menu(), Some(QuickRow::Exit));
    a.apply(Action::GbaDown(Btn::Down));
    assert_eq!(
        a.quick_menu(),
        Some(QuickRow::Options),
        "down from the bottom did not wrap"
    );
    a.apply(Action::GbaDown(Btn::Up));
    assert_eq!(
        a.quick_menu(),
        Some(QuickRow::Exit),
        "up from the top did not wrap"
    );
}

#[test]
fn b_leaves_the_menu_without_doing_anything() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::PowerHold);
    a.apply(Action::GbaDown(Btn::B));
    assert_eq!(a.quick_menu(), None);
    assert!(!a.powering_off() && !a.restarting() && !a.exiting());
    assert!(matches!(a.phase(), Phase::Shelf), "back to the shelf");
}

#[test]
fn each_row_commits_to_its_own_outcome() {
    for (row, want) in [
        (QuickRow::Restart, "restart"),
        (QuickRow::PowerOff, "off"),
        (QuickRow::Exit, "exit"),
    ] {
        let d = tmp_root_with_carts(&["Emerald"]);
        let mut a = boot(d.path());
        a.apply(Action::QuickMenu);
        for _ in 0..row.position() {
            a.apply(Action::GbaDown(Btn::Down));
        }
        assert_eq!(a.quick_menu(), Some(row));
        a.apply(Action::GbaDown(Btn::A));
        assert_eq!(
            a.quick_menu(),
            None,
            "{want}: the menu closes on the choice"
        );
        match want {
            "restart" => assert!(a.restarting() && !a.powering_off() && !a.exiting()),
            "off" => assert!(a.powering_off() && !a.restarting() && !a.exiting()),
            _ => assert!(a.exiting() && !a.powering_off() && !a.restarting()),
        }
    }
}

#[test]
fn the_shutdown_screen_is_up_before_the_machine_may_stop() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::PowerHold);
    a.apply(Action::GbaDown(Btn::A));

    assert!(a.powering_off(), "the choice decides immediately");
    assert!(
        !a.ready_to_power_off(),
        "but the binary may not act until the screen has been presented"
    );

    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(
        matches!(out.first(), Some(Draw::Rect { colour, .. }) if *colour == [0.0, 0.0, 0.0, 1.0]),
        "and the screen is what the loop is drawing in the meantime"
    );

    a.tick_ms(600_000);
    assert!(a.ready_to_power_off(), "then it may stop");
}

#[test]
fn holding_power_through_the_session_raises_the_menu() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now = 0;
    hold_power(&mut s, &mut now);
    assert_eq!(
        s.app().quick_menu(),
        Some(QuickRow::PowerOff),
        "the hold never raised the menu"
    );
}

#[test]
fn a_tap_on_power_does_nothing() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut s = session_with_platform(d.path());
    let mut now = 0;
    event(&mut s, RawEvent::Down(Btn::Power), &mut now);
    event(&mut s, RawEvent::Up(Btn::Power), &mut now);
    for _ in 0..80 {
        step(&mut s, &mut now);
    }
    assert_eq!(s.app().quick_menu(), None);
    assert!(matches!(s.app().phase(), Phase::Shelf));
    assert!(!s.app().powering_off());
}

#[test]
fn a_battery_at_the_last_gasp_powers_off() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut a, charge, percent) = common::app_with_charge(d.path());
    charge.store(1, Ordering::Relaxed);
    percent.store(4, Ordering::Relaxed);
    a.tick_ms(60_000);
    assert!(a.powering_off(), "a dying battery left the shelf running");
}

fn hold_power(s: &mut Session, now: &mut Millis) {
    let pressed = *now;
    event(s, RawEvent::Down(Btn::Power), now);
    while *now < pressed + POWER_HOLD_MS + FRAME_MS {
        step(s, now);
    }
}

fn step(s: &mut Session, now: &mut Millis) {
    *now += FRAME_MS;
    s.feed([], *now);
    s.update(DT);
}

fn event(s: &mut Session, ev: RawEvent, now: &mut Millis) {
    *now += FRAME_MS;
    s.feed([ev], *now);
    s.update(DT);
}
