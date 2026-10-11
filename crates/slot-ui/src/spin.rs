//! Random pick: the shelf spins along like a prize wheel, quick at first
//! and slowing, and comes to rest on the cart picked for you.

use crate::hud::Millis;

/// How long the spin takes from the press to the last click.
pub const SPIN_MS: Millis = 2200;
/// Fewest carts the shelf runs past, so even a near pick still spins.
pub const SPIN_MIN_STEPS: usize = 30;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Spin {
    started: Millis,
    steps: usize,
    taken: usize,
    target: usize,
}

impl Spin {
    /// A spin from `from` to `target` along a shelf of `n` carts.
    pub fn new(now: Millis, from: usize, target: usize, n: usize) -> Self {
        let ahead = (target + n - from) % n;
        let laps = SPIN_MIN_STEPS.saturating_sub(ahead).div_ceil(n.max(1));
        Spin {
            started: now,
            steps: ahead + laps * n,
            taken: 0,
            target,
        }
    }

    pub fn target(&self) -> usize {
        self.target
    }

    /// When the `k`th step falls: each gap longer than the last, so the
    /// shelf slows the way a wheel does.
    fn step_at(&self, k: usize) -> Millis {
        let f = k as f32 / self.steps as f32;
        self.started + (SPIN_MS as f32 * (1.0 - (1.0 - f).powf(2.2))) as Millis
    }

    /// How many steps are due by `now`, counted off as taken.
    pub fn due(&mut self, now: Millis) -> usize {
        let mut n = 0;
        while self.taken < self.steps && self.step_at(self.taken + 1) <= now {
            self.taken += 1;
            n += 1;
        }
        n
    }

    pub fn done(&self) -> bool {
        self.taken >= self.steps
    }
}
