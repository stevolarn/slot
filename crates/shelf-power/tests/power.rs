use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use shelf_power::{Battery, Charge, Platform, Power};

struct Panel {
    step: Arc<AtomicU8>,
    clock: i64,
}

impl Platform for Panel {
    fn set_backlight(&mut self, step: u8) {
        self.step.store(step, Ordering::Relaxed);
    }

    fn battery(&self) -> Option<Battery> {
        Some(Battery {
            percent: 42,
            charge: Charge::Charging,
        })
    }

    fn charge(&self) -> Charge {
        Charge::Charging
    }

    fn restart(&mut self) -> ! {
        panic!("the panel never powers off")
    }

    fn poweroff(&mut self) -> ! {
        panic!("the panel never powers off")
    }

    fn root(&self) -> &Path {
        Path::new("sdcard")
    }

    fn now(&self) -> i64 {
        self.clock
    }

    fn set_clock(&mut self, secs: i64) {
        self.clock = secs;
    }
}

fn power() -> (Power, Arc<AtomicU8>) {
    let step = Arc::new(AtomicU8::new(0));
    let panel = Panel {
        step: step.clone(),
        clock: 0,
    };
    (Power::new(Box::new(panel)), step)
}

#[test]
fn the_backlight_goes_straight_to_the_panel() {
    let (mut p, step) = power();
    p.set_backlight(7);
    assert_eq!(step.load(Ordering::Relaxed), 7);
    p.set_backlight(0);
    assert_eq!(step.load(Ordering::Relaxed), 0);
}

#[test]
fn the_battery_and_the_charge_come_from_the_platform() {
    let (p, _) = power();
    assert_eq!(
        p.battery(),
        Some(Battery {
            percent: 42,
            charge: Charge::Charging
        })
    );
    assert_eq!(p.charge(), Charge::Charging);
}

#[test]
fn the_clock_is_read_and_set_through_the_platform() {
    let (mut p, _) = power();
    assert_eq!(p.now(), 0);
    p.set_clock(1_700_000_000);
    assert_eq!(p.now(), 1_700_000_000);
}

#[test]
fn a_platform_that_says_nothing_has_a_writable_card() {
    let (p, _) = power();
    assert!(!p.card_read_only());
}
