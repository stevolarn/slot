//! Blowing on a cart, the way everyone did when a game would not start:
//! the cart hops and rocks for a moment and a puff of dust comes off its
//! top edge.

use slot_gfx::Draw;

use crate::hud::Millis;

pub const BLOW_MS: Millis = 700;

/// How long the rumble lasts, from the start of the puff.
pub const BLOW_RUMBLE_MS: Millis = 140;

const HOP_PX: f32 = 7.0;
const ROCK_PX: f32 = 4.0;
const ROCK_HZ: f32 = 9.0;

/// Where each speck of dust starts along the cart's top edge (as a share of
/// its width), which way it drifts, how big it is and how pale.
const SPECKS: [(f32, f32, f32, f32); 14] = [
    (0.10, -1.0, 5.0, 0.85),
    (0.18, -0.6, 3.0, 0.70),
    (0.26, -0.8, 4.0, 0.80),
    (0.33, -0.3, 6.0, 0.75),
    (0.40, -0.5, 3.0, 0.90),
    (0.46, -0.1, 5.0, 0.70),
    (0.52, 0.2, 4.0, 0.85),
    (0.58, 0.1, 3.0, 0.75),
    (0.64, 0.4, 6.0, 0.80),
    (0.70, 0.7, 4.0, 0.90),
    (0.76, 0.5, 3.0, 0.70),
    (0.83, 0.9, 5.0, 0.80),
    (0.90, 1.0, 4.0, 0.75),
    (0.50, 0.0, 7.0, 0.65),
];

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Blow {
    started: Millis,
}

impl Blow {
    pub fn started(now: Millis) -> Self {
        Blow { started: now }
    }

    pub fn active(&self, now: Millis) -> bool {
        now.saturating_sub(self.started) < BLOW_MS
    }

    pub fn rumbling(&self, now: Millis) -> bool {
        now.saturating_sub(self.started) < BLOW_RUMBLE_MS
    }

    fn progress(&self, now: Millis) -> Option<f32> {
        let age = now.saturating_sub(self.started);
        (age < BLOW_MS).then(|| age as f32 / BLOW_MS as f32)
    }

    /// How far the cart is pushed (x, y) at this moment: a quick hop up
    /// that settles, and a rocking side to side that dies away.
    pub fn jiggle(&self, now: Millis) -> (f32, f32) {
        let Some(t) = self.progress(now) else {
            return (0.0, 0.0);
        };
        let decay = 1.0 - t;
        let secs = t * BLOW_MS as f32 / 1000.0;
        let rock = (secs * ROCK_HZ * std::f32::consts::TAU).sin() * ROCK_PX * decay;
        let hop = -(t * std::f32::consts::PI * 2.0).sin().max(0.0) * HOP_PX;
        (rock, hop)
    }

    /// The dust coming off a cart whose top edge runs from `x` for `w`
    /// pixels at height `y`.
    pub fn draw_dust(&self, now: Millis, x: f32, y: f32, w: f32, out: &mut Vec<Draw>) {
        let Some(t) = self.progress(now) else {
            return;
        };
        let rise = 46.0 * (1.0 - (1.0 - t) * (1.0 - t));
        let fade = (1.0 - t) * (1.0 - t);
        for (along, drift, size, pale) in SPECKS {
            let size = size * (1.0 + t);
            out.push(Draw::Rect {
                x: x + along * w + drift * 30.0 * t - size / 2.0,
                y: y - 4.0 - rise * (0.6 + 0.4 * pale) - size / 2.0,
                w: size,
                h: size,
                colour: [pale * 0.86, pale * 0.82, pale * 0.74, 0.8 * fade],
            });
        }
    }
}
