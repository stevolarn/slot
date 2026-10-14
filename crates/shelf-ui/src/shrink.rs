//! A game nobody has rented yet comes shrink-wrapped, and one rented often is scuffed white along its edges.

use shelf_gfx::{Draw, TexId};

use crate::blow::{rgba as colour_of, unit};
use crate::plate::UndoFace;
use crate::text;

const WRAP_W: u32 = 96;
const WRAP_H: u32 = 144;
const SHEEN: [u8; 3] = [0xf4, 0xf8, 0xff];
/// Plays at which a box is as scuffed as it gets.
const WORN_OUT_PLAYS: f32 = 40.0;

/// How scuffed a game played `times` times is, 0 to 1: nothing for the
/// first couple of plays.
pub fn wear_of(times: u32) -> f32 {
    ((times as f32 - 2.0) / WORN_OUT_PLAYS).clamp(0.0, 1.0)
}

fn smooth(edge0: f32, edge1: f32, v: f32) -> f32 {
    let t = ((v - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The shrink-wrap, to be stretched over a box front: a faint veil, two
/// slanting streaks of gloss, crimped seams at top and bottom and a few
/// creases.
pub fn wrap_face() -> UndoFace {
    let (w, h) = (WRAP_W, WRAP_H);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
            let slant = fx + fy * 0.55;
            let band = |centre: f32, half: f32| 1.0 - smooth(0.0, half, (slant - centre).abs());
            let mut a = 26.0 + 105.0 * band(0.62, 0.17) + 75.0 * band(1.0, 0.05);
            let seam = y < 4 || y >= h - 4;
            if seam {
                // Crimped: a ridged strip, bright and dull in turn.
                a = if (x / 2 + y) % 2 == 0 { 96.0 } else { 44.0 };
            }
            if x == 0 || x == w - 1 {
                a = a.max(60.0);
            }
            // Creases: short slanted lines at fixed places.
            for k in 0..7u32 {
                let (cx, cy) = (unit(k * 3) * w as f32, 8.0 + unit(k * 3 + 1) * (h as f32 - 16.0));
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                let along = dx * 0.8 + dy * 0.6;
                let across = (-dx * 0.6 + dy * 0.8).abs();
                if along.abs() < 5.0 + 7.0 * unit(k * 3 + 2) && across < 0.7 {
                    a = a.max(70.0);
                }
            }
            text::blend_pixel(&mut rgba, w, x, y, a as u32, SHEEN);
        }
    }
    UndoFace { rgba, w, h }
}

/// Stretch the shrink-wrap `face` over the box at `rect`.
pub fn draw_wrap(face: TexId, (x, y, w, h): (f32, f32, f32, f32), alpha: f32, out: &mut Vec<Draw>) {
    out.push(Draw::Tex { x, y, w, h, tex: face, alpha });
}

/// How long the unwrapping takes, in ms, from the grab to the shine.
pub const PEEL_MS: u64 = 1100;
/// How long after A the box starts to open: once the wrap is off.
pub const PEEL_OPENS_AFTER_MS: u64 = 1100;

// The beats of the unwrapping, in ms from A.
const GRAB_END: f32 = 120.0;
const GLINT_END: f32 = 240.0;
const TEAR: (f32, f32) = (120.0, 380.0);
const PULL: (f32, f32) = (380.0, 850.0);
const REVEAL: (f32, f32) = (850.0, 1100.0);
/// Bits of plastic that pop off the tear.
const SPECKS: u32 = 9;
/// Pieces of the tear line along the top seam.
const TEAR_BITS: u32 = 16;

/// The wrap and its two halves, as uploaded: the halves fly off apart.
#[derive(Copy, Clone, Debug)]
pub struct WrapFaces {
    pub whole: TexId,
    pub left: TexId,
    pub right: TexId,
}

/// The wrap's left and right halves, each its own picture.
pub fn wrap_halves() -> (UndoFace, UndoFace) {
    let whole = wrap_face();
    let half = whole.w / 2;
    let cut = |from: u32| {
        let mut rgba = Vec::with_capacity((half * whole.h * 4) as usize);
        for y in 0..whole.h {
            let at = ((y * whole.w + from) * 4) as usize;
            rgba.extend_from_slice(&whole.rgba[at..at + (half * 4) as usize]);
        }
        UndoFace { rgba, w: half, h: whole.h }
    };
    (cut(0), cut(half))
}

fn ease(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    v * v * (3.0 - 2.0 * v)
}

fn span(ms: f32, (from, to): (f32, f32)) -> f32 {
    ((ms - from) / (to - from)).clamp(0.0, 1.0)
}

/// How much larger the box is drawn `ms` into the unwrapping: a squeeze as
/// it is grabbed and a little bounce once it is bare. 1 the rest of the time.
pub fn unwrap_squeeze(ms: u64) -> f32 {
    let ms = ms as f32;
    let pi = std::f32::consts::PI;
    if ms < GRAB_END {
        1.0 + 0.03 * (pi * ms / GRAB_END).sin()
    } else if (REVEAL.0..REVEAL.1).contains(&ms) {
        1.0 + 0.02 * (pi * span(ms, REVEAL)).sin()
    } else {
        1.0
    }
}

/// A box rect grown by `scale` about the middle of its foot, so it stands
/// where it stood.
pub fn grown((x, y, w, h): (f32, f32, f32, f32), scale: f32) -> (f32, f32, f32, f32) {
    let (gw, gh) = (w * scale, h * scale);
    (x + (w - gw) / 2.0, y + h - gh, gw, gh)
}

/// A light band `bw` wide with its middle at `mid`, kept inside the box.
fn band_in(
    (x, y, w, h): (f32, f32, f32, f32),
    mid: f32,
    bw: f32,
    alpha: f32,
    out: &mut Vec<Draw>,
) {
    let (l, r) = ((mid - bw / 2.0).max(x), (mid + bw / 2.0).min(x + w));
    if r > l && alpha > 0.0 {
        out.push(Draw::Rect { x: l, y, w: r - l, h, colour: [1.0, 1.0, 1.0, alpha] });
    }
}

/// Where the left and right halves are `pull` (0 to 1) of the way off the
/// box at `rect`: each half's rect, and how far it is turned, in radians.
pub fn half_poses(
    (x, y, w, h): (f32, f32, f32, f32),
    pull: f32,
) -> [((f32, f32, f32, f32), f32); 2] {
    let e = ease(pull);
    let (hw, hh) = (w / 2.0 * (1.0 - 0.35 * e), h * (1.0 - 0.45 * e));
    let lift = -h * (0.18 * e + 0.25 * e * e);
    [-1.0f32, 1.0].map(|side| {
        let home = x + w / 2.0 + side * w / 4.0;
        let cx = home + side * w * 0.6 * e;
        let cy = y + h / 2.0 + lift;
        let turn = side * (0.14 + 0.18 * e) * e;
        ((cx - hw / 2.0, cy - hh / 2.0, hw, hh), turn)
    })
}

/// The wrap coming off the shut box at `rect`, `ms` after A: a glint as
/// it is grabbed, the seam along the top tearing open with bits of
/// plastic popping off, the two halves pulled away apart, crumpling and
/// fading, and the bare box catching the light.
pub fn draw_unwrap(faces: WrapFaces, rect: (f32, f32, f32, f32), ms: u64, out: &mut Vec<Draw>) {
    if ms >= PEEL_MS {
        return;
    }
    let (x, y, w, h) = rect;
    let ms = ms as f32;
    let pull = span(ms, PULL);
    // Whole until it is pulled, then in two halves going their ways.
    if ms < PULL.0 {
        out.push(Draw::Tex { x, y, w, h, tex: faces.whole, alpha: 1.0 });
    } else if pull < 1.0 {
        let alpha = (1.0 - ease(pull)).powf(1.2);
        let [l, r] = half_poses(rect, pull);
        for ((hx, hy, hw, hh), turn, tex) in [(l.0, l.1, faces.left), (r.0, r.1, faces.right)] {
            out.push(Draw::Turned { x: hx, y: hy, w: hw, h: hh, tex, alpha, turn });
        }
    }
    // The glint as the box is picked up.
    if ms < GLINT_END {
        let g = ms / GLINT_END;
        let a = 0.32 * (std::f32::consts::PI * g).sin();
        band_in(rect, x - 0.2 * w + 1.4 * w * g, w * 0.14, a, out);
        band_in(rect, x - 0.2 * w + 1.4 * w * g, w * 0.32, a * 0.3, out);
    }
    // The tear, from the right hand corner to the left, along the seam.
    let tear = ease(span(ms, TEAR));
    let gone = ease(span(ms, (PULL.0, PULL.0 + 140.0)));
    if tear > 0.0 && gone < 1.0 {
        let bit = w / TEAR_BITS as f32;
        for k in 0..TEAR_BITS {
            if (k as f32 + 0.5) / TEAR_BITS as f32 > tear {
                break;
            }
            let jag = if k % 2 == 0 { -1.5 } else { 1.2 } + unit(k * 7) * 1.8;
            let (bx, by) = (x + w - bit * (k + 1) as f32, y + h * 0.06 + jag);
            let bh = 2.5 + unit(k * 7 + 1) * 2.0;
            let fade = 1.0 - gone;
            // A shadow under the torn edge, so it shows on a pale box too.
            out.push(Draw::Rect {
                x: bx,
                y: by + bh,
                w: bit + 0.5,
                h: 1.5,
                colour: [0.0, 0.0, 0.05, 0.45 * fade],
            });
            out.push(Draw::Rect {
                x: bx,
                y: by,
                w: bit + 0.5,
                h: bh,
                colour: [1.0, 1.0, 1.0, 0.95 * fade],
            });
        }
    }
    // Bits of plastic popping off the tear as it passes them.
    for j in 0..SPECKS {
        let along = (j as f32 + 0.5) / SPECKS as f32;
        let born = TEAR.0 + (TEAR.1 - TEAR.0) * along;
        let age = (ms - born) / 340.0;
        if !(0.0..1.0).contains(&age) {
            continue;
        }
        let vx = (unit(j * 13) - 0.5) * w * 0.35;
        let up = h * (0.10 + 0.10 * unit(j * 13 + 1));
        let size = 3.0 + 2.5 * unit(j * 13 + 2);
        out.push(Draw::Rect {
            x: x + w * (1.0 - along) + vx * age,
            y: y + h * 0.06 - up * age + h * 0.22 * age * age,
            w: size,
            h: size,
            colour: [1.0, 1.0, 1.0, 0.9 * (1.0 - age)],
        });
    }
    // The bare box catching the light: one bright sweep, and a twinkle at
    // the corners.
    let reveal = span(ms, REVEAL);
    if reveal > 0.0 {
        let pi = std::f32::consts::PI;
        let a = (pi * reveal).sin();
        let mid = x - 0.15 * w + 1.3 * w * ease(reveal);
        band_in(rect, mid, w * 0.12, 0.42 * a, out);
        band_in(rect, mid, w * 0.38, 0.12 * a, out);
        for (n, (cx, cy)) in [(0.12, 0.10), (0.9, 0.22), (0.18, 0.86), (0.84, 0.8)]
            .into_iter()
            .enumerate()
        {
            let t = ((reveal - n as f32 * 0.12) / 0.5).clamp(0.0, 1.0);
            let tw = (pi * t).sin();
            if tw <= 0.0 {
                continue;
            }
            let (sx, sy) = (x + w * cx, y + h * cy);
            let (long, thin) = (8.0 * tw + 2.0, 2.0);
            let colour = [1.0, 1.0, 0.92, 0.9 * tw];
            out.push(Draw::Rect { x: sx - long / 2.0, y: sy - thin / 2.0, w: long, h: thin, colour });
            out.push(Draw::Rect { x: sx - thin / 2.0, y: sy - long / 2.0, w: thin, h: long, colour });
        }
    }
}

/// Edges rubbed white on a box played `wear` (0 to 1): bands along its
/// sides and ends, and the corners worn through, more the more it was
/// handled, in the same places for the same `seed`.
pub fn draw_scuffs(
    (x, y, w, h): (f32, f32, f32, f32),
    wear: f32,
    alpha: f32,
    seed: u32,
    out: &mut Vec<Draw>,
) {
    let wear = wear.clamp(0.0, 1.0);
    if wear <= 0.0 || alpha <= 0.0 || w < 16.0 || h < 16.0 {
        return;
    }
    let base = seed.wrapping_mul(0x85eb_ca6b);
    let at = |k: u32| unit(base.wrapping_add(k));
    let edge = 0.8 + 3.2 * wear;
    let a = (0.10 + 0.34 * wear) * alpha;
    let c = [0.93, 0.92, 0.90];
    // Each side is rubbed in two or three stretches, not all along.
    for side in 0..4u32 {
        for run in 0..3u32 {
            let k = side * 10 + run * 3;
            if at(k) > 0.35 + 0.65 * wear {
                continue;
            }
            let span = if side < 2 { h } else { w };
            let len = span * (0.18 + 0.34 * at(k + 1));
            let from = (span - len) * at(k + 2);
            let r = match side {
                0 => (x, y + from, edge, len),
                1 => (x + w - edge, y + from, edge, len),
                2 => (x + from, y, len, edge),
                _ => (x + from, y + h - edge, len, edge),
            };
            out.push(Draw::Rect { x: r.0, y: r.1, w: r.2, h: r.3, colour: colour_of(c, a) });
        }
    }
    let corner = 3.0 + 7.0 * wear;
    for (cx, cy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        out.push(Draw::Rect {
            x: x + cx * (w - corner),
            y: y + cy * (h - corner),
            w: corner,
            h: corner,
            colour: colour_of(c, a * 1.1),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: (f32, f32, f32, f32) = (100.0, 50.0, 200.0, 300.0);

    fn faces() -> WrapFaces {
        WrapFaces {
            whole: TexId::from_raw(1),
            left: TexId::from_raw(2),
            right: TexId::from_raw(3),
        }
    }

    #[test]
    fn the_box_squeezes_and_bounces_and_is_still_otherwise() {
        assert_eq!(unwrap_squeeze(0), 1.0);
        assert!(unwrap_squeeze(60) > 1.02);
        assert_eq!(unwrap_squeeze(500), 1.0);
        assert!(unwrap_squeeze(975) > 1.01);
        assert_eq!(unwrap_squeeze(PEEL_MS), 1.0);
    }

    #[test]
    fn nothing_is_drawn_once_the_wrap_is_off() {
        let mut out = Vec::new();
        draw_unwrap(faces(), RECT, PEEL_MS, &mut out);
        assert!(out.is_empty());
        draw_unwrap(faces(), RECT, 10, &mut out);
        assert!(!out.is_empty());
    }

    #[test]
    fn the_halves_go_apart_and_up() {
        let [(l0, _), (r0, _)] = half_poses(RECT, 0.0);
        let [(l1, lt), (r1, rt)] = half_poses(RECT, 1.0);
        assert!(l1.0 < l0.0 && r1.0 > r0.0, "the halves did not part");
        assert!(l1.1 < l0.1 && r1.1 < r0.1, "the halves did not lift");
        assert!(lt < 0.0 && rt > 0.0, "the halves turn the same way");
    }

    #[test]
    fn grown_keeps_the_foot() {
        let g = grown(RECT, 1.1);
        assert!((g.1 + g.3 - (RECT.1 + RECT.3)).abs() < 1e-3);
        assert!((g.0 + g.2 / 2.0 - (RECT.0 + RECT.2 / 2.0)).abs() < 1e-3);
    }

    #[test]
    fn wear_grows_with_plays_and_stops() {
        assert_eq!(wear_of(0), 0.0);
        assert_eq!(wear_of(2), 0.0);
        assert!(wear_of(10) > wear_of(5));
        assert_eq!(wear_of(500), 1.0);
    }

    #[test]
    fn scuffs_grow_and_an_unplayed_box_has_none() {
        let n = |wear| {
            let mut out = Vec::new();
            draw_scuffs((0.0, 0.0, 200.0, 300.0), wear, 1.0, 7, &mut out);
            out.len()
        };
        assert_eq!(n(0.0), 0);
        assert!(n(1.0) > n(0.1));
    }
}
