//! Deciding when a frame need not be drawn and the loop may sleep.

use std::time::Duration;

/// How long the loop waits for input once the screen has settled. A press
/// ends the wait at once; this is how late a clock minute, a battery change
/// or a timed fade can be noticed.
pub const REST_TICK: Duration = Duration::from_millis(40);

/// The wait once the screen has stayed still a while longer: the shelf is
/// being looked at, or left, and waking 25 times a second only to find
/// nothing to draw costs battery. A press still ends it at once.
pub const LONG_REST_TICK: Duration = Duration::from_millis(250);

/// Frames in a row that drew nothing new, at [`REST_TICK`] apart, before
/// the longer wait: about three seconds.
const LONG_AFTER: u32 = SETTLED_AFTER + 75;

/// Frames in a row that drew nothing new before the screen counts as
/// settled: a few, so a spring's last wobble or a face still arriving does
/// not put the loop to sleep early.
const SETTLED_AFTER: u32 = 4;

/// What a frame was drawn at, besides its draw list: the lift and split of
/// the shelf frame and the size of the window.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Look {
    pub lift: f32,
    pub split: Option<f32>,
    pub window: (u32, u32),
}

/// A count of frames that matched the one on the screen.
#[derive(Default, Debug)]
pub struct Idle {
    same: u32,
}

impl Idle {
    /// A frame was drawn: the screen is moving.
    pub fn drawn(&mut self) {
        self.same = 0;
    }

    /// A frame matched what was on the screen, so nothing was drawn.
    pub fn skipped(&mut self) {
        self.same = self.same.saturating_add(1);
    }

    pub fn rested(&self) -> bool {
        self.same >= SETTLED_AFTER
    }

    /// How long the loop may wait for input while rested.
    pub fn tick(&self) -> Duration {
        if self.same >= LONG_AFTER {
            LONG_REST_TICK
        } else {
            REST_TICK
        }
    }
}
