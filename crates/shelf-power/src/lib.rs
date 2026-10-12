mod device;
mod power;
mod sim;

pub use device::{
    has_bit, read_only, read_only_in, record_first_frame, trace_first_frame, uptime_seconds,
    DevicePlatform,
};
pub use power::Power;
pub use sim::SimPlatform;

use std::path::Path;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Charge {
    Unknown,
    Discharging,
    Charging,
    Full,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Battery {
    pub percent: u8,
    pub charge: Charge,
}

pub trait Platform: Send {
    fn set_backlight(&mut self, step: u8);
    fn battery(&self) -> Option<Battery>;
    fn charge(&self) -> Charge;
    fn poweroff(&mut self) -> !;
    fn restart(&mut self) -> !;
    fn root(&self) -> &Path;
    fn now(&self) -> i64;
    fn set_clock(&mut self, secs: i64);

    fn card_read_only(&self) -> bool {
        false
    }
}
