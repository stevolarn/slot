//! Blowing on a cart, the way everyone did when a game would not start:
//! the cart lifts and tips its bottom towards you, showing the contacts
//! in its opening thick with dust, a breath goes over them and carries the
//! dust off, and the cart tips back down and settles.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, PI, TAU};

use shelf_gfx::Draw;

use crate::hud::Millis;
use crate::slot_chrome::ease;

pub const BLOW_MS: Millis = 1400;

/// Where each part of the blow ends, as a share of the whole.
/// The steps the store's cart bottom fades back in.
const FADE_BANDS: usize = 8;
const TIPPED: f32 = 0.2;
const BREATH_DONE: f32 = 0.7;
const DOWN: f32 = 0.88;

/// How far the cart lifts and tips at most.
const LIFT_PX: f32 = 24.0;
const TILT: f32 = FRAC_PI_3;
/// How thick a cart is, as a share of its width.
const DEPTH: f32 = 0.18;

const SHIVER_PX: f32 = 1.5;
const SHIVER_HZ: f32 = 14.0;
const HOP_PX: f32 = 5.0;
const ROCK_PX: f32 = 3.0;
const ROCK_HZ: f32 = 9.0;

const CONTACTS: u32 = 32;
const GOLD: [f32; 3] = [0.86, 0.68, 0.30];
const BOARD: [f32; 3] = [0.10, 0.27, 0.16];
const OPENING: [f32; 3] = [0.05, 0.05, 0.06];
pub(crate) const DUST: [f32; 3] = [0.62, 0.58, 0.52];
/// The specks of dust that sit on the plastic.
pub(crate) const SPECK: [f32; 3] = [0.80, 0.76, 0.68];
const BREATH: [f32; 3] = [0.93, 0.96, 1.0];

const DUST_SPECKS: u32 = 70;
/// How finely a turned-over disc's underside is cut into rows and columns.
const DISC_ROWS: u32 = 72;
const DISC_COLS: u32 = 18;
const HUB_ROWS: u32 = 40;
const DISC_SPECKS: u32 = 46;
/// How long a speck takes to blow away once the breath reaches it, as a
/// share of the breath.
const SPECK_LIFE: f32 = 0.35;
/// How fast the breath wipes across, in widths per breath.
const SWEEP: f32 = 1.6;

/// The puff of breath: each blob's place from the puff's middle and its
/// size, in puff radii, and how thick it is.
const PUFF: [(f32, f32, f32, f32, f32); 11] = [
    (0.0, 0.0, 2.2, 1.0, 1.0),
    (0.0, 0.0, 1.6, 1.5, 0.9),
    (0.0, 0.0, 1.0, 1.8, 0.8),
    (-0.9, 0.2, 1.2, 0.8, 0.7),
    (0.9, -0.2, 1.3, 0.9, 0.7),
    (-0.5, -0.5, 0.8, 0.7, 0.6),
    (0.5, 0.5, 0.9, 0.7, 0.6),
    (-1.4, 0.0, 0.6, 0.5, 0.5),
    (1.5, 0.1, 0.7, 0.5, 0.5),
    (0.2, -0.9, 0.6, 0.4, 0.4),
    (-0.2, 0.9, 0.6, 0.4, 0.4),
];

fn hsv(hue: f32, sat: f32) -> [f32; 3] {
    let h = hue / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    [r, g, b].map(|c| 1.0 - sat + sat * c)
}

/// A steady scatter in 0..1, the same every frame.
pub(crate) fn unit(i: u32) -> f32 {
    let mut h = i.wrapping_mul(0x9e37_79b9) ^ 0x5bd1_e995;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

pub(crate) fn rgba(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

/// How the cart blown on is held at a moment of the blow.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BlowPose {
    /// How far the cart is pushed, x and y.
    pub offset: (f32, f32),
    /// How much of its height its front still shows as it tips.
    pub stand: f32,
    /// How much of its thickness its bottom shows.
    pub bottom: f32,
    /// How much of its width a disc still shows as it turns over.
    pub squash: f32,
    /// Whether a disc has turned over far enough to show its underside.
    pub flipped: bool,
}

impl BlowPose {
    pub const STILL: BlowPose = BlowPose {
        offset: (0.0, 0.0),
        stand: 1.0,
        bottom: 0.0,
        squash: 1.0,
        flipped: false,
    };

    /// How high the cart's bottom shows when it is `w` wide.
    pub fn bottom_h(&self, w: f32) -> f32 {
        self.bottom * DEPTH * w
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Blow {
    started: Millis,
    disc: bool,
}

impl Blow {
    pub fn started(now: Millis) -> Self {
        Blow {
            started: now,
            disc: false,
        }
    }

    /// A blow on a disc: it is picked up and turned over to its shiny
    /// side, rather than tipped to show a cart's contacts.
    pub fn started_disc(now: Millis) -> Self {
        Blow {
            started: now,
            disc: true,
        }
    }

    pub fn is_disc(&self) -> bool {
        self.disc
    }

    pub fn active(&self, now: Millis) -> bool {
        now.saturating_sub(self.started) < BLOW_MS
    }

    fn progress(&self, now: Millis) -> Option<f32> {
        let age = now.saturating_sub(self.started);
        (age < BLOW_MS).then(|| age as f32 / BLOW_MS as f32)
    }

    /// How far tipped the cart is, 0 to 1.
    fn tip(t: f32) -> f32 {
        if t < TIPPED {
            ease(t / TIPPED)
        } else if t < BREATH_DONE {
            1.0
        } else if t < DOWN {
            1.0 - ease((t - BREATH_DONE) / (DOWN - BREATH_DONE))
        } else {
            0.0
        }
    }

    /// How far through the breath, 0 to 1, if it is going.
    fn breath(t: f32) -> Option<f32> {
        (TIPPED..BREATH_DONE)
            .contains(&t)
            .then(|| (t - TIPPED) / (BREATH_DONE - TIPPED))
    }

    /// How much of the dust on the front of the cart is left: all of it
    /// until the breath, then the breath carries it off.
    pub fn dust_left(&self, now: Millis) -> f32 {
        let Some(t) = self.progress(now) else {
            return 1.0;
        };
        if t < TIPPED {
            1.0
        } else if t < BREATH_DONE {
            1.0 - ease((t - TIPPED) / (BREATH_DONE - TIPPED))
        } else {
            0.0
        }
    }

    pub fn pose(&self, now: Millis) -> BlowPose {
        let Some(t) = self.progress(now) else {
            return BlowPose::STILL;
        };
        let tip = Self::tip(t);
        let secs = t * BLOW_MS as f32 / 1000.0;
        let (mut x, mut y) = (0.0, -LIFT_PX * tip);
        if let Some(b) = Self::breath(t) {
            x += (secs * SHIVER_HZ * TAU).sin() * SHIVER_PX * (b * PI).sin();
        }
        if t >= DOWN {
            let s = (t - DOWN) / (1.0 - DOWN);
            y -= (s * PI).sin() * HOP_PX * (1.0 - s);
            let since = (t - DOWN) * BLOW_MS as f32 / 1000.0;
            x += (since * ROCK_HZ * TAU).sin() * ROCK_PX * (1.0 - s);
        }
        if self.disc {
            let turn = tip * PI;
            return BlowPose {
                offset: (x, y),
                stand: 1.0,
                bottom: 0.0,
                squash: turn.cos().abs(),
                flipped: turn > FRAC_PI_2,
            };
        }
        BlowPose {
            offset: (x, y),
            stand: (tip * TILT).cos(),
            bottom: (tip * TILT).sin(),
            squash: 1.0,
            flipped: false,
        }
    }

    /// How far the breath has wiped across the cart's bottom, 0 to 1.
    fn swept(t: f32) -> f32 {
        if t < TIPPED {
            0.0
        } else {
            Self::breath(t).map_or(1.0, |b| (b * SWEEP).min(1.0))
        }
    }

    /// The cart's bottom, `x`, `y`, `w` wide and `h` high, seen as it
    /// tips: its end in `shell` plastic and the opening across `pins` of
    /// its width with the board's contacts in it, under dust the breath
    /// wipes away.
    pub fn draw_bottom(
        &self,
        now: Millis,
        (x, y, w, h): (f32, f32, f32, f32),
        shell: [u8; 3],
        pins: (f32, f32),
        fade: bool,
        out: &mut Vec<Draw>,
    ) {
        let Some(t) = self.progress(now) else {
            return;
        };
        if h < 1.0 {
            return;
        }
        let plastic = shell.map(|c| c as f32 / 255.0 * 0.72);
        let lip = shell.map(|c| (c as f32 / 255.0 * 1.1 + 0.06).min(1.0));
        // How solid the bottom is `v` of the way down it: with `fade`, out
        // in the open store, it thins into what is behind as it goes back,
        // rather than ending in a hard dark edge.
        let solid = |v: f32| if fade { 1.0 - 0.9 * v } else { 1.0 };
        if fade {
            for k in 0..FADE_BANDS {
                let v = (k as f32 + 0.5) / FADE_BANDS as f32;
                let inset = w * 0.04 * (k + 1) as f32 / FADE_BANDS as f32;
                out.push(Draw::Rect {
                    x: x + inset,
                    y: y + h * k as f32 / FADE_BANDS as f32,
                    w: w - 2.0 * inset,
                    h: h / FADE_BANDS as f32,
                    colour: rgba(plastic, solid(v)),
                });
            }
        } else {
            out.push(Draw::Rect {
                x,
                y,
                w,
                h,
                colour: rgba(plastic, 1.0),
            });
        }
        out.push(Draw::Rect {
            x,
            y,
            w,
            h: (h * 0.08).max(1.0),
            colour: rgba(lip, 1.0),
        });

        let (ox, ow) = (x + w * pins.0, w * (pins.1 - pins.0));
        let (oy, oh) = (y + h * 0.25, h * 0.55);
        out.push(Draw::Rect {
            x: ox,
            y: oy,
            w: ow,
            h: oh,
            colour: rgba(OPENING, solid(0.5)),
        });
        let (by, bh) = (oy + oh * 0.3, oh * 0.4);
        out.push(Draw::Rect {
            x: ox + 2.0,
            y: by,
            w: ow - 4.0,
            h: bh,
            colour: rgba(BOARD, solid(0.5)),
        });
        let pitch = (ow - 4.0) / CONTACTS as f32;
        let pin_w = (pitch * 0.55).max(1.0);
        for i in 0..CONTACTS {
            out.push(Draw::Rect {
                x: (ox + 2.0 + pitch * i as f32 + (pitch - pin_w) / 2.0).round(),
                y: by,
                w: pin_w,
                h: bh,
                colour: rgba(GOLD, 1.0),
            });
        }

        // The dust the breath has not reached yet.
        let swept = Self::swept(t);
        let clean_to = x + w * swept;
        if swept < 1.0 {
            out.push(Draw::Rect {
                x: clean_to,
                y,
                w: x + w - clean_to,
                h,
                colour: rgba(DUST, 0.55 * solid(0.5)),
            });
        }
        let blown = Self::breath(t).map_or(if t < TIPPED { 0.0 } else { 1.0 }, |b| b);
        for i in 0..DUST_SPECKS {
            let (u, v, size) = (unit(i * 3), unit(i * 3 + 1), 2.0 + 3.0 * unit(i * 3 + 2));
            let sx = x + u * (w - size);
            let sy = y + v * (h - size);
            // When the breath gets to it, it lifts off and blows away.
            let age = (blown - u / SWEEP) / SPECK_LIFE;
            if age >= 1.0 {
                continue;
            }
            let (sx, sy, size, a) = if age > 0.0 {
                let up = 0.5 + unit(i * 3 + 1);
                (
                    sx + age * 110.0,
                    sy - age * age * 90.0 * up,
                    size * (1.0 + age),
                    0.8 * (1.0 - age),
                )
            } else {
                (sx, sy, size, 0.8)
            };
            out.push(Draw::Rect {
                x: sx.round(),
                y: sy.round(),
                w: size,
                h: size,
                colour: [0.80, 0.76, 0.68, a],
            });
        }
    }

    /// The shiny underside of a disc turned over, in the `x`, `y`, `w` by
    /// `h` box the disc fills, `squash` of its width showing: a mirror with
    /// a rainbow sheen and a clear hub, fingerprints and dust on it that the
    /// breath wipes off, and a glint running over it once it is clean.
    pub fn draw_underside(
        &self,
        now: Millis,
        (x, y, w, h): (f32, f32, f32, f32),
        squash: f32,
        out: &mut Vec<Draw>,
    ) {
        let Some(t) = self.progress(now) else {
            return;
        };
        if squash <= 0.01 || w < 1.0 {
            return;
        }
        let (cx, cy, r) = (x + w / 2.0, y + h / 2.0, 0.47 * w);
        let swept = Self::swept(t);
        let glint = if t >= BREATH_DONE {
            ((t - BREATH_DONE) / (DOWN - BREATH_DONE)).min(1.0)
        } else {
            -1.0
        };
        for k in 0..DISC_ROWS {
            let dy = ((k as f32 + 0.5) / DISC_ROWS as f32 * 2.0 - 1.0) * r;
            let chord = (r * r - dy * dy).max(0.0).sqrt();
            let row_h = 2.0 * r / DISC_ROWS as f32 + 0.6;
            let row_y = cy + dy - row_h / 2.0;
            for j in 0..DISC_COLS {
                let (a, b) = (j as f32 / DISC_COLS as f32, (j + 1) as f32 / DISC_COLS as f32);
                let dx = ((a + b) - 1.0) * chord;
                let theta = dy.atan2(dx);
                let sheen = (0.5 + 0.5 * (2.0 * (theta - 0.7)).cos()).powi(2);
                let mut c = [0.64 + 0.28 * sheen; 3];
                let rainbow = hsv((theta.to_degrees() * 2.0).rem_euclid(360.0), 0.34);
                for n in 0..3 {
                    c[n] = c[n] * 0.62 + rainbow[n] * 0.38;
                }
                // The glint: a bright band running left to right.
                let at = (dx / r + 1.0) / 2.0;
                if glint >= 0.0 {
                    let g = (1.0 - ((at - glint * 1.4 + 0.2) / 0.12).abs()).max(0.0);
                    for n in c.iter_mut() {
                        *n = (*n + g * 0.5).min(1.0);
                    }
                }
                // Greasy until the breath has gone over it.
                if at > swept {
                    for n in c.iter_mut() {
                        *n = *n * 0.82 + 0.1;
                    }
                }
                out.push(Draw::Rect {
                    x: cx + (-chord + 2.0 * chord * a) * squash,
                    y: row_y,
                    w: 2.0 * chord * (b - a) * squash + 0.6,
                    h: row_h,
                    colour: rgba(c, 1.0),
                });
            }
        }
        // The clear hub, and the ring round it.
        for (radius, shade, alpha) in [(0.2, [0.45, 0.46, 0.5], 1.0), (0.1, [0.04, 0.04, 0.05], 0.9)] {
            let hr = radius * w;
            for k in 0..HUB_ROWS {
                let dy = ((k as f32 + 0.5) / HUB_ROWS as f32 * 2.0 - 1.0) * hr;
                let chord = (hr * hr - dy * dy).max(0.0).sqrt() * squash;
                out.push(Draw::Rect {
                    x: cx - chord,
                    y: cy + dy - hr / HUB_ROWS as f32,
                    w: 2.0 * chord,
                    h: 2.0 * hr / HUB_ROWS as f32 + 0.6,
                    colour: rgba(shade, alpha),
                });
            }
        }
        // Fingerprint smudges and dust, lifting off where the breath gets to them.
        let blown = Self::breath(t).map_or(if t < TIPPED { 0.0 } else { 1.0 }, |b| b);
        for i in 0..DISC_SPECKS {
            let (u, v) = (unit(i * 4 + 900), unit(i * 4 + 901));
            let (size, up) = (2.0 + 5.0 * unit(i * 4 + 902), 0.5 + unit(i * 4 + 903));
            // A point in the circle, outside the hub.
            let (ang, rad) = (u * TAU, 0.24 + 0.74 * v.sqrt());
            let (px, py) = (ang.cos() * rad, ang.sin() * rad);
            let age = (blown - (px + 1.0) / 2.0 / SWEEP) / SPECK_LIFE;
            if age >= 1.0 {
                continue;
            }
            let (lift_x, lift_y, grow, a) = if age > 0.0 {
                (age * 90.0, -age * age * 70.0 * up, 1.0 + age, 0.7 * (1.0 - age))
            } else {
                (0.0, 0.0, 1.0, 0.7)
            };
            let (sx, sy) = (cx + px * r * squash + lift_x, cy + py * r + lift_y);
            out.push(Draw::Rect {
                x: (sx - size * grow / 2.0).round(),
                y: (sy - size * grow / 2.0).round(),
                w: (size * grow * squash).max(1.0),
                h: size * grow,
                colour: rgba(if i % 3 == 0 { DUST } else { SPECK }, a),
            });
        }
    }

    /// The breath going over the cart's bottom at `x`, `y`, `w` wide and
    /// `h` high: a soft puff that comes in from in front, swells and
    /// drifts across it, and thins away.
    pub fn draw_breath(
        &self,
        now: Millis,
        (x, y, w, h): (f32, f32, f32, f32),
        out: &mut Vec<Draw>,
    ) {
        let Some(b) = self.progress(now).and_then(Self::breath) else {
            return;
        };
        let along = (b * SWEEP).min(1.0);
        let cx = x + w * (-0.1 + 1.0 * along);
        let cy = y + h * 0.5 + 40.0 * (1.0 - b).powi(2);
        let r = (w * 0.13 + h * 0.5) * (0.6 + 0.9 * b);
        let thick = (b * PI).sin().powf(0.6) * 0.3;
        for (dx, dy, bw, bh, a) in PUFF {
            let (bw, bh) = (bw * r, bh * r * 0.75);
            out.push(Draw::Rect {
                x: cx + dx * r - bw / 2.0,
                y: cy + dy * r * 0.75 - bh / 2.0,
                w: bw,
                h: bh,
                colour: rgba(BREATH, thick * a),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(t: f32) -> Millis {
        (t * BLOW_MS as f32) as Millis
    }

    #[test]
    fn the_cart_starts_and_ends_still() {
        let blow = Blow::started(0);
        assert_eq!(blow.pose(0), BlowPose::STILL);
        assert_eq!(blow.pose(BLOW_MS), BlowPose::STILL);
        let late = blow.pose(BLOW_MS - 1);
        assert!(late.offset.0.abs() < 0.5 && late.offset.1.abs() < 0.5);
        assert_eq!(late.bottom, 0.0);
    }

    #[test]
    fn the_bottom_shows_while_it_is_blown_on() {
        let blow = Blow::started(0);
        let tipped = blow.pose(at(0.45));
        assert!(tipped.bottom > 0.8, "{tipped:?}");
        assert!(tipped.stand < 0.6, "{tipped:?}");
        assert!(tipped.offset.1 < -LIFT_PX + 1.0);
        assert!(blow.pose(at(0.1)).bottom < tipped.bottom);
    }

    #[test]
    fn the_breath_carries_off_the_dust_on_the_front() {
        let blow = Blow::started(0);
        assert_eq!(blow.dust_left(0), 1.0);
        assert_eq!(blow.dust_left(at(TIPPED - 0.01)), 1.0);
        let mid = blow.dust_left(at(0.45));
        assert!(mid > 0.0 && mid < 1.0, "{mid}");
        assert!(blow.dust_left(at(0.6)) < mid, "the dust came back");
        assert_eq!(blow.dust_left(at(BREATH_DONE + 0.01)), 0.0);
        assert_eq!(blow.dust_left(at(0.95)), 0.0);
    }

    #[test]
    fn a_disc_turns_over_instead_of_tipping() {
        let blow = Blow::started_disc(0);
        assert_eq!(blow.pose(0), BlowPose::STILL);
        assert_eq!(blow.pose(BLOW_MS), BlowPose::STILL);
        let mid = blow.pose(at(0.45));
        assert!(mid.flipped && mid.squash > 0.95, "{mid:?}");
        assert_eq!((mid.stand, mid.bottom), (1.0, 0.0));
        let edge = blow.pose(at(0.1));
        assert!(!edge.flipped && edge.squash < 1.0, "{edge:?}");
        assert!(!blow.pose(at(0.95)).flipped);
    }

    #[test]
    fn the_breath_wipes_a_discs_underside_clean() {
        let blow = Blow::started_disc(0);
        let rect = (0.0, 0.0, 120.0, 120.0);
        let specks = |t| {
            let mut out = Vec::new();
            blow.draw_underside(at(t), rect, 1.0, &mut out);
            out.iter()
                .filter(|d| matches!(d, Draw::Rect { colour, .. } if colour[3] < 0.99 && colour[3] > 0.0 && colour[0] > 0.55 && colour[0] < 0.85 && colour[1] > 0.5))
                .count()
        };
        assert_eq!(specks(0.21), DISC_SPECKS as usize);
        assert!(specks(0.45) < DISC_SPECKS as usize);
        assert_eq!(specks(0.69), 0);
    }

    #[test]
    fn the_breath_clears_all_the_dust() {
        let blow = Blow::started(0);
        let rect = (0.0, 0.0, 300.0, 50.0);
        let dust = |t| {
            let mut out = Vec::new();
            blow.draw_bottom(at(t), rect, [0x40; 3], (0.1, 0.9), false, &mut out);
            out.iter()
                .filter(|d| matches!(d, Draw::Rect { colour, .. } if colour[0] > 0.75 && colour[2] > 0.6 && colour[2] < 0.7))
                .count()
        };
        assert_eq!(dust(0.21), DUST_SPECKS as usize);
        assert!(dust(0.45) < DUST_SPECKS as usize);
        assert_eq!(dust(0.69), 0);
        let mut puff = Vec::new();
        blow.draw_breath(at(0.45), rect, &mut puff);
        assert!(!puff.is_empty());
        puff.clear();
        blow.draw_breath(at(0.8), rect, &mut puff);
        assert!(puff.is_empty());
    }
}
