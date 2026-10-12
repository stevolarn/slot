use crate::{Battery, Charge, Platform};

pub struct Power {
    platform: Box<dyn Platform>,
}

impl Power {
    pub fn new(platform: Box<dyn Platform>) -> Self {
        Power { platform }
    }

    pub fn set_backlight(&mut self, step: u8) {
        self.platform.set_backlight(step);
    }

    pub fn suspend(&mut self) -> crate::Suspend {
        self.platform.suspend()
    }

    pub fn battery(&self) -> Option<Battery> {
        self.platform.battery()
    }

    pub fn charge(&self) -> Charge {
        self.platform.charge()
    }

    pub fn card_read_only(&self) -> bool {
        self.platform.card_read_only()
    }

    pub fn poweroff(&mut self) -> ! {
        self.platform.poweroff()
    }

    pub fn restart(&mut self) -> ! {
        self.platform.restart()
    }

    pub fn now(&self) -> i64 {
        self.platform.now()
    }

    pub fn set_clock(&mut self, secs: i64) {
        self.platform.set_clock(secs);
    }
}
