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

/// How long the wrap takes to come off, in ms.
pub const PEEL_MS: u64 = 700;

/// The wrap coming off the box at `rect`, `t` (0 to 1) of the way: pulled
/// up off the front and shrinking away as it goes, fading out.
pub fn draw_peel(face: TexId, (x, y, w, h): (f32, f32, f32, f32), t: f32, out: &mut Vec<Draw>) {
    let t = t.clamp(0.0, 1.0);
    if t >= 1.0 {
        return;
    }
    let e = t * t * (3.0 - 2.0 * t);
    let (pw, ph) = (w * (1.0 - 0.25 * e), h * (1.0 - 0.55 * e));
    out.push(Draw::Tex {
        x: x + (w - pw) / 2.0,
        y: y - h * 0.45 * e,
        w: pw,
        h: ph,
        tex: face,
        alpha: (1.0 - e).powf(1.3),
    });
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
