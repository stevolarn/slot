mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::App;
use shelf_input::{Action, Btn};
use shelf_store::{read_receipts, start_playing, write_shelf_state, ShelfState};
use shelf_ui::QuickRow;

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn monotonic_secs() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime only writes the timespec it is given.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as u64
}

/// A card that has just had Alpha out for ten minutes.
fn back_from_alpha(receipts: bool) -> (tempfile::TempDir, App) {
    back_from_alpha_in(receipts, None)
}

/// The same, with Alpha handed out in the boot `boot`.
fn back_from_alpha_in(receipts: bool, boot: Option<&str>) -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Alpha", "Bravo"]);
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            receipts,
            ..ShelfState::default()
        },
    )
    .unwrap();
    let rom = d.path().join("Games/GBA/Alpha.gba");
    start_playing(d.path(), &rom, monotonic_secs() - 600, 0, boot).unwrap();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    for _ in 0..20 {
        a.update(0.1);
    }
    (d, a)
}

#[test]
fn a_game_coming_back_prints_a_receipt_and_logs_it() {
    let (d, mut a) = back_from_alpha(true);
    let logged = read_receipts(d.path());
    assert_eq!(logged.len(), 1, "the return was not logged");
    assert!((600..=602).contains(&logged[0].secs), "{} s", logged[0].secs);
    let slip = a.slip().expect("no receipt printed");
    assert!(slip.at.is_some() && slip.torn.is_none());
    let (Some(r), _) = a.receipt_wanted().expect("no receipt face wanted") else {
        panic!("an empty receipt");
    };
    assert_eq!(r.title, "Alpha");
    assert_eq!(r.number, 1);
    assert_eq!(shelf_ui::slip_rows(&r)[1][0], ("PLAYED", "10M".to_string()), "the plain slip");
    // Any button tears it off and does nothing else.
    press(&mut a, Btn::Start);
    assert!(a.quick_menu().is_none(), "the press reached the shelf");
    for _ in 0..10 {
        a.update(0.1);
    }
    assert!(a.slip().is_none(), "the receipt stayed after it was torn off");
}

#[test]
fn with_receipts_off_the_return_is_logged_but_nothing_prints() {
    let (d, a) = back_from_alpha(false);
    assert_eq!(read_receipts(d.path()).len(), 1);
    assert!(a.slip().is_none());
}

#[test]
fn the_spike_shows_the_newest_first_and_down_goes_back_in_time() {
    let (_d, mut a) = back_from_alpha(false);
    a.apply(Action::QuickMenu);
    while a.quick_menu() != Some(QuickRow::Receipts) {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    let (Some(r), _) = a.receipt_wanted().expect("nothing on the spike") else {
        panic!("an empty spike");
    };
    assert_eq!(r.number, 1);
    press(&mut a, Btn::Down);
    assert!(a.receipt_wanted().is_some_and(|k| k.0.is_some()), "went past the last");
    press(&mut a, Btn::B);
    assert_eq!(a.quick_menu(), Some(QuickRow::Receipts));
}

#[test]
fn a_game_handed_out_before_a_restart_gets_no_receipt() {
    if shelf_store::boot_id().is_none() {
        return;
    }
    let (d, a) = back_from_alpha_in(true, Some("an-earlier-boot"));
    assert!(read_receipts(d.path()).is_empty());
    assert!(a.slip().is_none());
}
