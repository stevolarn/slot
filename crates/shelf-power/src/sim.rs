use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Battery, Charge, Platform};

pub struct SimPlatform {
    root: PathBuf,
    offset: i64,
    stopped: Option<i64>,
}

impl SimPlatform {
    pub fn new() -> Self {
        SimPlatform::at(root_from_env())
    }

    pub fn at(root: PathBuf) -> Self {
        SimPlatform {
            root,
            offset: 0,
            stopped: None,
        }
    }

    pub fn stopped_at(mut self, secs: i64) -> Self {
        self.stopped = Some(secs);
        self
    }
}

impl Default for SimPlatform {
    fn default() -> Self {
        SimPlatform::new()
    }
}

impl Platform for SimPlatform {
    fn set_backlight(&mut self, _step: u8) {}

    fn battery(&self) -> Option<Battery> {
        Some(Battery {
            percent: 100,
            charge: Charge::Charging,
        })
    }

    fn charge(&self) -> Charge {
        Charge::Charging
    }

    fn restart(&mut self) -> ! {
        std::process::exit(0)
    }

    fn poweroff(&mut self) -> ! {
        std::process::exit(0)
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn now(&self) -> i64 {
        self.stopped.unwrap_or_else(|| system_secs() + self.offset)
    }

    fn set_clock(&mut self, secs: i64) {
        match &mut self.stopped {
            Some(stopped) => *stopped = secs,
            None => self.offset = secs - system_secs(),
        }
    }
}

fn system_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn root_from_env() -> PathBuf {
    std::env::var_os("SHELF_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sdcard"))
}
