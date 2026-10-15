mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use common::tmp_root_with_carts;
use shelf::frontend::sleep_until_power_press;
use shelf::session::Session;
use shelf_input::{Btn, InputSource, Millis, RawEvent, POWER_HOLD_MS, POWER_SPURIOUS_MS};
use shelf_power::{Battery, Charge, Platform, Power, Suspend};

type Log = Arc<Mutex<Vec<String>>>;

struct Mock {
    root: PathBuf,
    log: Log,
    outcome: Suspend,
}

impl Platform for Mock {
    fn set_backlight(&mut self, step: u8) {
        self.log.lock().unwrap().push(format!("backlight {step}"));
    }
    fn suspend(&mut self) -> Suspend {
        self.log.lock().unwrap().push("suspend".into());
        self.outcome
    }
    fn battery(&self) -> Option<Battery> {
        None
    }
    fn charge(&self) -> Charge {
        Charge::Unknown
    }
    fn poweroff(&mut self) -> ! {
        panic!("powered off")
    }
    fn restart(&mut self) -> ! {
        panic!("restarted")
    }
    fn root(&self) -> &Path {
        &self.root
    }
    fn now(&self) -> i64 {
        1_800_000_000
    }
    fn set_clock(&mut self, _secs: i64) {}
}

fn session(log: &Log) -> (tempfile::TempDir, Session) {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut s = Session::boot(d.path().to_path_buf());
    let mock = Mock {
        root: d.path().to_path_buf(),
        log: log.clone(),
        outcome: Suspend::Woke,
    };
    s.app_mut().set_power(Power::new(Box::new(mock)));
    log.lock().unwrap().clear();
    (d, s)
}

fn press(s: &mut Session, ev: RawEvent, now: Millis) {
    s.feed([ev], now);
    s.update(1.0 / 60.0);
}

#[test]
fn a_tap_on_power_asks_to_sleep_and_a_hold_opens_the_menu_instead() {
    let log = Log::default();
    let (_d, mut s) = session(&log);
    press(&mut s, RawEvent::Down(Btn::Power), 0);
    press(&mut s, RawEvent::Up(Btn::Power), 120);
    assert!(s.app_mut().take_sleep(), "a tap did not ask to sleep");
    assert!(!s.app_mut().take_sleep(), "the ask was not cleared");

    press(&mut s, RawEvent::Down(Btn::Power), 1_000);
    s.feed([], 1_000 + POWER_HOLD_MS);
    s.update(1.0 / 60.0);
    press(&mut s, RawEvent::Up(Btn::Power), 3_000);
    assert!(!s.app_mut().take_sleep(), "a hold also slept");
    assert!(
        s.app().quick_menu().is_some(),
        "a hold did not open the menu"
    );
}

#[test]
fn sleeping_turns_the_screen_off_suspends_and_turns_it_back_on() {
    let log = Log::default();
    let (_d, mut s) = session(&log);
    let brightness = s.app().brightness();
    assert_eq!(s.app_mut().suspend(), Suspend::Woke);
    s.app_mut().wake();
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "backlight 0".to_string(),
            "suspend".to_string(),
            format!("backlight {brightness}")
        ]
    );
}

#[test]
fn power_presses_in_the_second_after_waking_do_nothing() {
    let log = Log::default();
    let (_d, mut s) = session(&log);
    s.ignore_power_until(10_000 + POWER_SPURIOUS_MS);
    press(&mut s, RawEvent::Down(Btn::Power), 10_050);
    press(&mut s, RawEvent::Up(Btn::Power), 10_100);
    assert!(!s.app_mut().take_sleep(), "a spurious press slept again");
    press(&mut s, RawEvent::Down(Btn::Power), 12_000);
    press(&mut s, RawEvent::Up(Btn::Power), 12_080);
    assert!(s.app_mut().take_sleep(), "a later tap was ignored");
}

struct Scripted(Vec<Vec<RawEvent>>);

impl InputSource for Scripted {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        if self.0.is_empty() {
            Vec::new()
        } else {
            self.0.remove(0)
        }
    }
}

#[test]
fn without_a_suspend_script_the_screen_stays_off_until_a_power_press() {
    let mut input = Scripted(vec![
        vec![],
        vec![RawEvent::Down(Btn::A), RawEvent::Up(Btn::A)],
        vec![RawEvent::Down(Btn::Power)],
        vec![RawEvent::Down(Btn::B)],
    ]);
    sleep_until_power_press(&mut input, || 0);
    assert_eq!(input.0.len(), 1, "it woke before the power press, or after");
}
