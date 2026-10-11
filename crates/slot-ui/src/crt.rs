//! CRT mode: the shelf as an old TV shows it, with dark scanlines, a
//! picture that darkens towards its edges and rounded corners where the
//! curved glass meets the bezel.

use crate::draw::{Draw, OUT_H, OUT_W};

pub const SCANLINE_PITCH: u32 = 3;
pub const SCANLINE: [f32; 4] = [0.0, 0.0, 0.0, 0.28];
const CORNER_R: f32 = 36.0;
const VIGNETTE_STEPS: u32 = 10;
const VIGNETTE_PX: f32 = 4.0;

pub fn draw_crt(out: &mut Vec<Draw>) {
    let (w, h) = (OUT_W as f32, OUT_H as f32);
    for y in (0..OUT_H).step_by(SCANLINE_PITCH as usize) {
        out.push(Draw::Rect {
            x: 0.0,
            y: y as f32,
            w,
            h: 1.0,
            colour: SCANLINE,
        });
    }
    // Rings that get darker towards the edge, so the middle stays bright.
    for i in 0..VIGNETTE_STEPS {
        let inset = i as f32 * VIGNETTE_PX;
        let a = 0.05 * (1.0 - i as f32 / VIGNETTE_STEPS as f32);
        let colour = [0.0, 0.0, 0.0, a];
        let (iw, ih) = (w - 2.0 * inset, h - 2.0 * inset);
        for (x, y, rw, rh) in [
            (inset, inset, iw, VIGNETTE_PX),
            (inset, h - inset - VIGNETTE_PX, iw, VIGNETTE_PX),
            (
                inset,
                inset + VIGNETTE_PX,
                VIGNETTE_PX,
                ih - 2.0 * VIGNETTE_PX,
            ),
            (
                w - inset - VIGNETTE_PX,
                inset + VIGNETTE_PX,
                VIGNETTE_PX,
                ih - 2.0 * VIGNETTE_PX,
            ),
        ] {
            out.push(Draw::Rect {
                x,
                y,
                w: rw,
                h: rh,
                colour,
            });
        }
    }
    // The corners, masked off a row at a time along a quarter circle.
    for row in 0..CORNER_R as u32 {
        let dy = CORNER_R - row as f32 - 0.5;
        let cut = (CORNER_R - (CORNER_R * CORNER_R - dy * dy).max(0.0).sqrt()).round();
        if cut <= 0.0 {
            continue;
        }
        for (x, y) in [
            (0.0, row as f32),
            (w - cut, row as f32),
            (0.0, h - 1.0 - row as f32),
            (w - cut, h - 1.0 - row as f32),
        ] {
            out.push(Draw::Rect {
                x,
                y,
                w: cut,
                h: 1.0,
                colour: [0.0, 0.0, 0.0, 1.0],
            });
        }
    }
}
