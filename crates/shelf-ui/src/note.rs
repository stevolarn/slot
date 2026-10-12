//! The note left in a game's box when it has a save to go back to: a
//! scrap of lined paper with CONTINUE? scribbled on it, tucked in behind
//! the box so its end pokes out of the side, and lying on the cart once it
//! is out.

use shelf_gfx::{Draw, TexId};

use crate::plate::UndoFace;
use crate::rental::blit_turned;
use crate::text;

const NOTE_W: u32 = 104;
const NOTE_H: u32 = 52;
/// How far the note is turned, clockwise, in degrees.
const TILT: f32 = -7.0;
/// How much room the turned note needs round it.
const MARGIN: u32 = 10;

const PAPER: [u8; 3] = [0xf7, 0xf1, 0xd6];
const LINE: [u8; 3] = [0x9c, 0xb8, 0xe0];
const MARGIN_LINE: [u8; 3] = [0xe0, 0x6a, 0x6a];
const INK: [u8; 3] = [0x24, 0x2c, 0x70];

/// How much of the note's width pokes out from the side of its box.
pub const NOTE_SHOWS: f32 = 0.8;

/// The note, upright.
fn paper() -> Vec<u8> {
    let (w, h) = (NOTE_W, NOTE_H);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            // A torn top edge: a few pixels nibbled out along it.
            let tear = (x.wrapping_mul(7919) ^ x.wrapping_mul(131)) % 3;
            if y < tear {
                continue;
            }
            let c = if x == 12 || x == 13 {
                MARGIN_LINE
            } else if y > 8 && (y - 8) % 11 == 0 {
                LINE
            } else {
                PAPER
            };
            text::blend_pixel(&mut buf, w, x, y, 255, c);
        }
    }
    if let Some(font) = text::label_font() {
        let lay = text::fit(font, "CONTINUE?", (w - 22) as f32, 1, 15.0, 9.0);
        if let Some(line) = lay.lines.first() {
            let tw = text::line_width(font, line, lay.px, lay.tracking);
            let x = 16.0 + ((w as f32 - 20.0) - tw) / 2.0;
            let base = h as f32 * 0.48 + text::cap_height(font, lay.px) / 2.0;
            text::draw_line(&mut buf, w, h, x, base, line, lay.px, lay.tracking, INK, 255);
            // Underlined, twice, in a hurry.
            let under = (base + 4.0) as u32;
            for (dy, from, to) in [(0, 0.0, 1.0), (3, 0.1, 0.8)] {
                let y = under + dy;
                if y >= h {
                    continue;
                }
                let (a, b) = (x + tw * from, x + tw * to);
                for px in a as u32..(b as u32).min(w) {
                    text::blend_pixel(&mut buf, w, px, y, 220, INK);
                }
            }
        }
    }
    buf
}

/// The note, turned a little, with a soft shadow, on a clear face.
pub fn continue_note_face() -> UndoFace {
    let (w, h) = (NOTE_W + 2 * MARGIN, NOTE_H + 2 * MARGIN);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let note = paper();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let size = (NOTE_W, NOTE_H);
    blit_turned(&mut rgba, w, h, &note, size, (cx + 2.0, cy + 3.0), TILT, Some(([0, 0, 0], 70)));
    blit_turned(&mut rgba, w, h, &note, size, (cx, cy), TILT, None);
    UndoFace { rgba, w, h }
}

/// The note tucked behind a box or cart at `x`, `y`, `w` wide: its end
/// poking out of the right hand side, near the top, clear of the game's
/// name above. Draw it before the box, so the box hides the rest of it.
/// `face` is the note and its size, `scale` how large to draw it.
pub fn draw_note_behind(
    face: (TexId, u32, u32),
    (x, y, w): (f32, f32, f32),
    scale: f32,
    alpha: f32,
    out: &mut Vec<Draw>,
) {
    let (tex, fw, fh) = face;
    let (nw, nh) = (fw as f32 * scale, fh as f32 * scale);
    out.push(Draw::Tex {
        x: (x + w - nw * (1.0 - NOTE_SHOWS)).round(),
        y: (y + 6.0).round(),
        w: nw,
        h: nh,
        tex,
        alpha,
    });
}

/// The note lying on a cart at `x`, `y`: over its top left corner.
pub fn draw_note_on(face: (TexId, u32, u32), (x, y): (f32, f32), out: &mut Vec<Draw>) {
    let (tex, fw, fh) = face;
    out.push(Draw::Tex {
        x: (x - 8.0).round(),
        y: (y - fh as f32 * 0.35).round(),
        w: fw as f32,
        h: fh as f32,
        tex,
        alpha: 1.0,
    });
}
