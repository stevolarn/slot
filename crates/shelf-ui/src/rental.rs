//! The rental shop's sticker on the back of a box: how many times the
//! game has been played and when last, on a yellow label slapped on at
//! a slant.

use shelf_store::civil_from_days;

use crate::info::blit_turned;
use crate::text;

const DAY: i64 = 86_400;
const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// How long the label is for how tall it is.
const ASPECT: f32 = 2.3;

/// What the sticker says: plays, and the day of the last one.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RentalSticker {
    pub times: u32,
    pub month: u8,
    pub day: u8,
    /// PLAYED rather than RENTED, for the original theme.
    pub plain: bool,
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
            plain: false,
        })
    }

    /// The sticker in plain words when `plain`.
    pub fn worded(self, plain: bool) -> Self {
        Self { plain, ..self }
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
        format!("{} {}{times}", if r.plain { "PLAYED" } else { "RENTED" }, r.times),
        format!("LAST PLAYED {} {month}", r.day),
    )
}

/// How long a game has been played for, as the tags say it: "4H 20M",
/// "35M", or nothing under a minute.
pub fn play_time_text(secs: u64) -> Option<String> {
    let mins = secs / 60;
    match (mins / 60, mins % 60) {
        (0, 0) => None,
        (0, m) => Some(format!("{m}M")),
        (h, 0) => Some(format!("{h}H")),
        (h, m) => Some(format!("{h}H {m}M")),
    }
}

/// How big the sticker is when it is `sh` tall.
pub fn rental_size(sh: f32) -> (f32, f32) {
    (sh * ASPECT, sh)
}

const OVERDUE_RED: [u8; 3] = [0xd4, 0x2a, 0x2a];
const OVERDUE_W: u32 = 92;
const OVERDUE_H: u32 = 40;
const OVERDUE_MARGIN: u32 = 8;

/// The OVERDUE label, upright.
fn overdue_label() -> Vec<u8> {
    let (w, h) = (OVERDUE_W, OVERDUE_H);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let (fw, fh) = (w as f32, h as f32);
    let cut = (fh * 0.12).max(1.0);
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let corner = [(0.0, 0.0), (fw, 0.0), (0.0, fh), (fw, fh)]
                .iter()
                .any(|(cx, cy)| (px - cx).abs() + (py - cy).abs() < cut);
            if !corner {
                text::blend_pixel(&mut buf, w, x, y, 255, OVERDUE_RED);
            }
        }
    }
    let Some(font) = text::label_font() else {
        return buf;
    };
    let pad = fh * 0.12;
    let room = fw - 2.0 * pad;
    let mut centred = |words: &str, px: f32, mid: f32| {
        let lay = text::fit(font, words, room, 1, px, px * 0.5);
        if let Some(line) = lay.lines.first() {
            let tw = text::line_width(font, line, lay.px, lay.tracking);
            let base = mid + text::cap_height(font, lay.px) / 2.0;
            text::draw_line(
                &mut buf, w, h, (fw - tw) / 2.0, base, line, lay.px, lay.tracking,
                [255, 255, 255], 255,
            );
        }
    };
    centred("OVERDUE", fh * 0.46, fh * 0.34);
    centred("LATE FEES APPLY", fh * 0.17, fh * 0.80);
    let rule_y = (fh * 0.60).round() as u32;
    let rule_h = (fh * 0.04).round().max(1.0) as u32;
    for y in rule_y..(rule_y + rule_h).min(h) {
        for x in (pad as u32)..((fw - pad) as u32).min(w) {
            text::blend_pixel(&mut buf, w, x, y, 255, [255, 255, 255]);
        }
    }
    buf
}

/// The OVERDUE sticker, turned a little, with a soft shadow, on a clear face.
pub fn overdue_face() -> crate::plate::UndoFace {
    let (w, h) = (OVERDUE_W + 2 * OVERDUE_MARGIN, OVERDUE_H + 2 * OVERDUE_MARGIN);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let label = overdue_label();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let size = (OVERDUE_W, OVERDUE_H);
    blit_turned(&mut rgba, w, h, &label, size, (cx + 2.0, cy + 3.0), 8.0, Some(([0, 0, 0], 80)));
    blit_turned(&mut rgba, w, h, &label, size, (cx, cy), 8.0, None);
    crate::plate::UndoFace { rgba, w, h }
}

/// Stick the OVERDUE `face` on the item at `rect`, over its lower right
/// corner, `frac` of the item's width across, at `alpha`.
pub fn draw_overdue(
    face: (shelf_gfx::TexId, u32, u32),
    (x, y, w, h): (f32, f32, f32, f32),
    frac: f32,
    alpha: f32,
    out: &mut Vec<shelf_gfx::Draw>,
) {
    let (tex, fw, fh) = face;
    let dw = w * frac;
    let dh = dw * fh as f32 / fw as f32;
    out.push(shelf_gfx::Draw::Tex {
        x: (x + w - dw * 0.8).round(),
        y: (y + h - dh * 0.85).round(),
        w: dw,
        h: dh,
        tex,
        alpha,
    });
}

