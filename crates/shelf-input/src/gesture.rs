use crate::{Btn, Millis, RawEvent};

pub const SELECT_CHORD_MS: Millis = 600;

pub const SELECT_TAP_MS: Millis = 50;
pub const MUTE_CHORD_MS: Millis = 200;
pub const POWER_HOLD_MS: Millis = 1000;

pub const VOLUME_REPEAT_DELAY_MS: Millis = 400;
pub const VOLUME_REPEAT_MS: Millis = 120;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Action {
    GbaDown(Btn),
    GbaUp(Btn),
    ShelfLeft,
    ShelfRight,
    BrightnessUp,
    BrightnessDown,
    VolumeUp,
    VolumeDown,
    QuickMenu,
    MuteToggle,
    PowerHold,
    /// The power button pressed and let go before the hold: sleep.
    PowerTap,
}

/// How long power presses are ignored after the device wakes: the key that
/// woke it, and the odd extra press the power chip reports just after.
pub const POWER_SPURIOUS_MS: Millis = 1000;

#[derive(Copy, Clone, Default)]
enum Select {
    #[default]
    Idle,
    Held {
        since: Millis,
        chorded: bool,
        sent: Option<Millis>,
    },
    ReleaseDue(Millis),
}

#[derive(Default)]
pub struct Gestures {
    select: Select,
    chord_held: u16,
    power_down_at: Option<Millis>,
    power_hold_fired: bool,
    vol_up_at: Option<Millis>,
    vol_down_at: Option<Millis>,
    vol_up_ramp: Option<Millis>,
    vol_down_ramp: Option<Millis>,
    mute_fired: bool,
    power_ignored_until: Option<Millis>,
    /// Menu is down: the volume buttons set the brightness, as in NextUI,
    /// and the menu opens on letting go only if they were not used.
    menu_held: bool,
    menu_chorded: bool,
}

fn ramp_due(down: Option<Millis>, last: Option<Millis>, now: Millis) -> bool {
    let Some(down) = down else {
        return false;
    };
    if now.saturating_sub(down) < VOLUME_REPEAT_DELAY_MS {
        return false;
    }
    last.is_none_or(|l| now.saturating_sub(l) >= VOLUME_REPEAT_MS)
}

impl Gestures {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ignore the power button until `until`, as after waking from sleep.
    /// A press already begun is forgotten.
    pub fn ignore_power_until(&mut self, until: Millis) {
        self.power_ignored_until = Some(until);
        self.power_down_at = None;
        self.power_hold_fired = false;
    }

    pub fn feed(&mut self, ev: RawEvent, now: Millis) -> Vec<Action> {
        if matches!(ev, RawEvent::Down(Btn::Power) | RawEvent::Up(Btn::Power))
            && self.power_ignored_until.is_some_and(|until| now < until)
        {
            return Vec::new();
        }
        match ev {
            RawEvent::Down(b) => self.down(b, now),
            RawEvent::Up(b) => self.up(b, now),
        }
    }

    /// Whether a button is held or a timer is running that a later tick
    /// will act on, so the caller must keep ticking.
    pub fn pending(&self) -> bool {
        !matches!(self.select, Select::Idle)
            || self.power_down_at.is_some()
            || self.vol_up_at.is_some()
            || self.vol_down_at.is_some()
    }

    pub fn tick(&mut self, now: Millis) -> Vec<Action> {
        let mut out = Vec::new();
        if let Select::ReleaseDue(due) = self.select {
            if now >= due {
                self.select = Select::Idle;
                out.push(Action::GbaUp(Btn::Select));
            }
        }
        if let Select::Held {
            since,
            chorded: false,
            sent: None,
        } = self.select
        {
            if now.saturating_sub(since) >= SELECT_CHORD_MS {
                out.extend(self.hand_over_select(now));
            }
        }
        if let Some(d) = self.power_down_at {
            if !self.power_hold_fired && now.saturating_sub(d) >= POWER_HOLD_MS {
                self.power_hold_fired = true;
                out.push(Action::PowerHold);
            }
        }
        if !self.mute_fired {
            let (up, down) = match self.menu_held {
                true => (Action::BrightnessUp, Action::BrightnessDown),
                false => (Action::VolumeUp, Action::VolumeDown),
            };
            if ramp_due(self.vol_up_at, self.vol_up_ramp, now) {
                self.vol_up_ramp = Some(now);
                out.push(up);
            }
            if ramp_due(self.vol_down_at, self.vol_down_ramp, now) {
                self.vol_down_ramp = Some(now);
                out.push(down);
            }
        }
        out
    }

    fn down(&mut self, b: Btn, now: Millis) -> Vec<Action> {
        if let (true, Some((bit, action))) = (self.chording(now), chord(b)) {
            self.mark_chorded();
            self.chord_held |= bit;
            return vec![action];
        }
        match b {
            Btn::Select => self.select_down(now),
            Btn::Menu => {
                self.menu_held = true;
                self.menu_chorded = false;
                Vec::new()
            }
            Btn::Power => {
                self.power_down_at = Some(now);
                self.power_hold_fired = false;
                Vec::new()
            }
            Btn::VolUp | Btn::VolDown => self.volume_press(b, now),
            _ => {
                let mut out = self.hand_over_select(now);
                out.push(Action::GbaDown(b));
                out
            }
        }
    }

    fn up(&mut self, b: Btn, now: Millis) -> Vec<Action> {
        if let Some((bit, _)) = chord(b) {
            if self.chord_held & bit != 0 {
                self.chord_held &= !bit;
                return Vec::new();
            }
        }
        match b {
            Btn::Select => self.select_up(now),
            Btn::Menu => {
                let open = self.menu_held && !self.menu_chorded;
                self.menu_held = false;
                self.menu_chorded = false;
                match open {
                    true => vec![Action::QuickMenu],
                    false => Vec::new(),
                }
            }
            Btn::Power => self.power_up(),
            Btn::VolUp | Btn::VolDown => self.volume_release(b),
            _ => vec![Action::GbaUp(b)],
        }
    }

    fn chording(&self, now: Millis) -> bool {
        match self.select {
            Select::Held { since, chorded, .. } => {
                chorded || now.saturating_sub(since) < SELECT_CHORD_MS
            }
            _ => false,
        }
    }

    fn mark_chorded(&mut self) {
        if let Select::Held { chorded, .. } = &mut self.select {
            *chorded = true;
        }
    }

    fn select_down(&mut self, now: Millis) -> Vec<Action> {
        let mut out = Vec::new();
        if matches!(self.select, Select::ReleaseDue(_)) {
            out.push(Action::GbaUp(Btn::Select));
        }
        if !matches!(self.select, Select::Held { .. }) {
            self.select = Select::Held {
                since: now,
                chorded: false,
                sent: None,
            };
        }
        out
    }

    fn hand_over_select(&mut self, now: Millis) -> Vec<Action> {
        match &mut self.select {
            Select::Held {
                chorded: false,
                sent: sent @ None,
                ..
            } => {
                *sent = Some(now);
                vec![Action::GbaDown(Btn::Select)]
            }
            _ => Vec::new(),
        }
    }

    fn select_up(&mut self, now: Millis) -> Vec<Action> {
        let Select::Held { chorded, sent, .. } = std::mem::take(&mut self.select) else {
            return Vec::new();
        };
        match (sent, chorded) {
            (Some(at), _) if now.saturating_sub(at) >= SELECT_TAP_MS => {
                vec![Action::GbaUp(Btn::Select)]
            }
            (Some(at), _) => {
                self.select = Select::ReleaseDue(at + SELECT_TAP_MS);
                Vec::new()
            }
            (None, true) => Vec::new(),
            (None, false) => {
                self.select = Select::ReleaseDue(now + SELECT_TAP_MS);
                vec![Action::GbaDown(Btn::Select)]
            }
        }
    }

    fn power_up(&mut self) -> Vec<Action> {
        let tapped = self.power_down_at.is_some() && !self.power_hold_fired;
        self.power_down_at = None;
        self.power_hold_fired = false;
        if tapped {
            vec![Action::PowerTap]
        } else {
            Vec::new()
        }
    }

    fn volume_press(&mut self, b: Btn, now: Millis) -> Vec<Action> {
        let (mine, other, action) = match b {
            Btn::VolUp => (&mut self.vol_up_at, self.vol_down_at, Action::VolumeUp),
            _ => (&mut self.vol_down_at, self.vol_up_at, Action::VolumeDown),
        };
        *mine = Some(now);
        match b {
            Btn::VolUp => self.vol_up_ramp = None,
            _ => self.vol_down_ramp = None,
        }
        if self.menu_held {
            self.menu_chorded = true;
            return vec![match b {
                Btn::VolUp => Action::BrightnessUp,
                _ => Action::BrightnessDown,
            }];
        }
        let mut out = vec![action];
        let paired = other.is_some_and(|t| now.abs_diff(t) <= MUTE_CHORD_MS);
        if paired && !self.mute_fired {
            self.mute_fired = true;
            out.push(Action::MuteToggle);
        }
        out
    }

    fn volume_release(&mut self, b: Btn) -> Vec<Action> {
        match b {
            Btn::VolUp => (self.vol_up_at, self.vol_up_ramp) = (None, None),
            _ => (self.vol_down_at, self.vol_down_ramp) = (None, None),
        }
        if self.vol_up_at.is_none() && self.vol_down_at.is_none() {
            self.mute_fired = false;
        }
        Vec::new()
    }
}

fn chord(b: Btn) -> Option<(u16, Action)> {
    Some(match b {
        Btn::Up => (1, Action::BrightnessUp),
        Btn::Down => (2, Action::BrightnessDown),
        Btn::VolUp => (4, Action::MuteToggle),
        Btn::VolDown => (8, Action::MuteToggle),
        _ => return None,
    })
}

#[cfg(test)]
mod menu_brightness {
    use super::*;

    #[test]
    fn menu_with_the_volume_buttons_sets_the_brightness_and_opens_no_menu() {
        let mut g = Gestures::new();
        assert!(g.feed(RawEvent::Down(Btn::Menu), 0).is_empty());
        assert_eq!(g.feed(RawEvent::Down(Btn::VolUp), 10), [Action::BrightnessUp]);
        g.feed(RawEvent::Up(Btn::VolUp), 20);
        assert_eq!(g.feed(RawEvent::Down(Btn::VolDown), 30), [Action::BrightnessDown]);
        g.feed(RawEvent::Up(Btn::VolDown), 40);
        assert!(g.feed(RawEvent::Up(Btn::Menu), 50).is_empty(), "the menu opened too");
        g.feed(RawEvent::Down(Btn::Menu), 100);
        assert_eq!(g.feed(RawEvent::Up(Btn::Menu), 150), [Action::QuickMenu]);
        assert_eq!(g.feed(RawEvent::Down(Btn::VolUp), 200), [Action::VolumeUp]);
    }
}
