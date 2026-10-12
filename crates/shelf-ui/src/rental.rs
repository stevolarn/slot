//! The rental shop's sticker on the back of a box: how many times the
//! game has been played and when last, on a yellow label slapped on at
//! a slant.

use shelf_store::civil_from_days;

use crate::text;

const DAY: i64 = 86_400;
const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

const LABEL: [u8; 3] = [0xff, 0xd3, 0x2a];
const RULE: [u8; 3] = [0x1b, 0x3a, 0x8c];
const INK: [u8; 3] = [0x14, 0x1e, 0x4a];
/// How long the label is for how tall it is.
const ASPECT: f32 = 2.3;

/// What the sticker says: plays, and the day of the last one.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RentalSticker {
    pub times: u32,
    pub month: u8,
    pub day: u8,
}

impl RentalSticker {
    /// The sticker for a game played `times` times, last at `last_wall`
    /// (unix seconds on the wall clock), or none for a game never played.
    pub fn new(times: u32, last_wall: i64) -> Option<Self> {
        if times == 0 {
            return None;
        }
        let (_, month, day) = civil_from_days(last_wall.div_euclid(DAY));
        Some(Self {
            times,
            month: month as u8,
            day: day as u8,
        })
    }
}

/// "RENTED 12×" and "LAST PLAYED 3 OCT".
pub fn rental_lines(r: &RentalSticker) -> (String, String) {
    let times = match text::label_font() {
        Some(f) if f.lookup_glyph_index('\u{d7}') != 0 => '\u{d7}',
        _ => 'X',
    };
    let month = MONTHS
        .get(usize::from(r.month).wrapping_sub(1))
        .copied()
        .unwrap_or_default();
    (
        format!("RENTED {}{times}", r.times),
        format!("LAST PLAYED {} {month}", r.day),
    )
}

/// How big the sticker is when it is `sh` tall.
pub fn rental_size(sh: f32) -> (f32, f32) {
    (sh * ASPECT, sh)
}

/// The label, upright, `w` by `h`.
fn label(r: &RentalSticker, w: u32, h: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let (fw, fh) = (w as f32, h as f32);
    // Clipped corners, like a sticker off a roll.
    let cut = (fh * 0.12).max(1.0);
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let corner = [(0.0, 0.0), (fw, 0.0), (0.0, fh), (fw, fh)]
                .iter()
                .any(|(cx, cy)| (px - cx).abs() + (py - cy).abs() < cut);
            if !corner {
                text::blend_pixel(&mut buf, w, x, y, 255, LABEL);
            }
        }
    }
    let Some(font) = text::label_font() else {
        return buf;
    };
    let pad = fh * 0.12;
    let room = fw - 2.0 * pad;
    let (top, bottom) = rental_lines(r);
    let mut centred = |words: &str, px: f32, mid: f32| {
        let lay = text::fit(font, words, room, 1, px, px * 0.5);
        if let Some(line) = lay.lines.first() {
            let tw = text::line_width(font, line, lay.px, lay.tracking);
            let base = mid + text::cap_height(font, lay.px) / 2.0;
            text::draw_line(
                &mut buf,
                w,
                h,
                (fw - tw) / 2.0,
                base,
                line,
                lay.px,
                lay.tracking,
                INK,
                255,
            );
        }
    };
    centred(&top, fh * 0.40, fh * 0.36);
    centred(&bottom, fh * 0.21, fh * 0.76);
    // A blue rule between the two lines.
    let rule_y = (fh * 0.57).round() as u32;
    let rule_h = (fh * 0.035).round().max(1.0) as u32;
    for y in rule_y..(rule_y + rule_h).min(h) {
        for x in (pad as u32)..((fw - pad) as u32).min(w) {
            text::blend_pixel(&mut buf, w, x, y, 255, RULE);
        }
    }
    buf
}

/// Blend `src` (`sw` by `sh`) onto `dst` turned `deg` degrees clockwise
/// about its middle, which lands at (cx, cy); in `tint` at `alpha`/255
/// instead of its own colours when given, for a shadow.
#[allow(clippy::too_many_arguments)]
fn blit_turned(
    dst: &mut [u8],
    dw: u32,
    dh: u32,
    src: &[u8],
    (sw, sh): (u32, u32),
    (cx, cy): (f32, f32),
    deg: f32,
    tint: Option<([u8; 3], u32)>,
) {
    let (sin, cos) = deg.to_radians().sin_cos();
    let (hw, hh) = (sw as f32 / 2.0, sh as f32 / 2.0);
    let reach = (hw * hw + hh * hh).sqrt().ceil() + 1.0;
    let x0 = (cx - reach).floor().max(0.0) as u32;
    let y0 = (cy - reach).floor().max(0.0) as u32;
    let x1 = ((cx + reach).ceil().max(0.0) as u32).min(dw);
    let y1 = ((cy + reach).ceil().max(0.0) as u32).min(dh);
    let texel = |x: i32, y: i32| -> [u32; 4] {
        if x < 0 || y < 0 || x >= sw as i32 || y >= sh as i32 {
            return [0; 4];
        }
        let at = ((y as u32 * sw + x as u32) * 4) as usize;
        let a = src[at + 3] as u32;
        [
            src[at] as u32 * a,
            src[at + 1] as u32 * a,
            src[at + 2] as u32 * a,
            a,
        ]
    };
    for y in y0..y1 {
        for x in x0..x1 {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            // Turn back the other way to find where in the label this is.
            let u = dx * cos + dy * sin + hw - 0.5;
            let v = -dx * sin + dy * cos + hh - 0.5;
            if u < -1.0 || v < -1.0 || u > sw as f32 || v > sh as f32 {
                continue;
            }
            let (ux, vy) = (u.floor(), v.floor());
            let (fx, fy) = (u - ux, v - vy);
            let (ix, iy) = (ux as i32, vy as i32);
            let mut sum = [0.0f32; 4];
            for (tx, ty, wgt) in [
                (ix, iy, (1.0 - fx) * (1.0 - fy)),
                (ix + 1, iy, fx * (1.0 - fy)),
                (ix, iy + 1, (1.0 - fx) * fy),
                (ix + 1, iy + 1, fx * fy),
            ] {
                let t = texel(tx, ty);
                for c in 0..4 {
                    sum[c] += t[c] as f32 * wgt;
                }
            }
            let a = sum[3];
            if a < 0.5 {
                continue;
            }
            let (colour, alpha) = match tint {
                Some((colour, strength)) => (colour, a as u32 * strength / 255),
                None => (
                    [0, 1, 2].map(|c| (sum[c] / a).round().clamp(0.0, 255.0) as u8),
                    a.round() as u32,
                ),
            };
            text::blend_pixel(dst, dw, x, y, alpha.min(255), colour);
        }
    }
}

/// Stick the sticker for `r`, `sh` tall, onto `dst` with its middle at
/// (cx, cy), turned `deg` degrees clockwise, with a soft shadow under it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_rental(
    dst: &mut [u8],
    dw: u32,
    dh: u32,
    r: &RentalSticker,
    (cx, cy): (f32, f32),
    sh: f32,
    deg: f32,
) {
    let (lw, lh) = rental_size(sh);
    let (lw, lh) = (lw.round().max(1.0) as u32, lh.round().max(1.0) as u32);
    let src = label(r, lw, lh);
    let off = (sh * 0.05).max(1.0);
    blit_turned(
        dst,
        dw,
        dh,
        &src,
        (lw, lh),
        (cx + off, cy + off * 1.5),
        deg,
        Some(([0, 0, 0], 90)),
    );
    blit_turned(dst, dw, dh, &src, (lw, lh), (cx, cy), deg, None);
}
