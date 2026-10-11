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

/// How far through the puff the film is all gone.
const FILM_CLEARS: f32 = 0.6;
const FILM_SPECKS: u32 = 90;

/// A steady scatter in 0..1, the same every frame.
fn unit(i: u32) -> f32 {
    let mut h = i.wrapping_mul(0x9e37_79b9) ^ 0x5bd1_e995;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

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

    /// The dust sitting on the cart when you go to blow on it, cleared away
    /// by the puff: a grey film and a scatter of specks over the cart that
    /// is at `x`, `y`, `w` wide and `h` high.
    pub fn draw_film(&self, now: Millis, (x, y, w, h): (f32, f32, f32, f32), out: &mut Vec<Draw>) {
        let Some(t) = self.progress(now) else {
            return;
        };
        let clear = (t / FILM_CLEARS).min(1.0);
        let (x, y, w, h) = (x + w * 0.04, y + h * 0.03, w * 0.92, h * 0.94);
        if clear < 1.0 {
            out.push(Draw::Rect {
                x,
                y,
                w,
                h,
                colour: [0.62, 0.58, 0.52, 0.42 * (1.0 - clear)],
            });
        }
        for i in 0..FILM_SPECKS {
            let (u, v, gone, size) = (
                unit(i * 4),
                unit(i * 4 + 1),
                0.15 + 0.85 * unit(i * 4 + 2),
                2.0 + 3.0 * unit(i * 4 + 3),
            );
            if clear >= gone {
                continue;
            }
            out.push(Draw::Rect {
                x: (x + u * (w - size)).round(),
                y: (y + v * (h - size)).round(),
                w: size,
                h: size,
                colour: [0.80, 0.76, 0.68, 0.75],
            });
        }
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
