//! The membership card: the About screen's front, a laminated card with
//! what Shelf knows of your playing. The video store's is the shop's
//! membership card in its blue and yellow; the original theme's a plain
//! grey player card. A turns it over to the small print.

use shelf_store::civil_from_days;

use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::plate::UndoFace;
use crate::text;

pub const CARD_W: u32 = 500;
pub const CARD_H: u32 = 316;
const RADIUS: f32 = 22.0;
const BAND_H: u32 = 78;
const PAD: f32 = 30.0;
const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// What the card says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberCard {
    /// The original theme's plain player card, not the shop's.
    pub plain: bool,
    /// The first play Shelf or NextUI knows of, in unix seconds on the
    /// wall clock; 0 for none.
    pub since: i64,
    pub plays: u32,
    pub secs: u64,
    pub genre: Option<String>,
    /// The build's serial, printed under the barcode.
    pub serial: String,
}

struct Look {
    body: [u8; 3],
    band: [u8; 3],
    band_ink: [u8; 3],
    label: [u8; 3],
    value: [u8; 3],
    title: &'static str,
    corner: &'static str,
    plays: &'static str,
}

fn look(plain: bool) -> Look {
    match plain {
        false => Look {
            body: [0x1b, 0x3a, 0x8c],
            band: [0xff, 0xd3, 0x2a],
            band_ink: [0x14, 0x1e, 0x4a],
            label: [0xff, 0xd3, 0x2a],
            value: [0xf6, 0xf4, 0xef],
            title: "SHELF VIDEO",
            corner: "MEMBER",
            plays: "RENTALS",
        },
        true => Look {
            body: [0x2c, 0x2c, 0x31],
            band: [0xc9, 0xc9, 0xc7],
            band_ink: [0x2a, 0x2a, 0x2e],
            label: [0x9a, 0x9a, 0xa4],
            value: [0xf6, 0xf4, 0xef],
            title: "SHELF",
            corner: "PLAYER CARD",
            plays: "PLAYS",
        },
    }
}

/// "MAR 2025", or a dash before any play.
pub fn since_text(since: i64) -> String {
    if since <= 0 {
        return "\u{2014}".to_string();
    }
    let (year, month, _) = civil_from_days(since.div_euclid(86_400));
    let month = MONTHS.get((month - 1) as usize).copied().unwrap_or_default();
    format!("{month} {year}")
}

/// "96", "4H 20M" under ten hours, "0" before any play.
pub fn hours_text(secs: u64) -> String {
    match secs / 3600 {
        h if h >= 10 => h.to_string(),
        _ => crate::play_time_text(secs).unwrap_or_else(|| "0".to_string()),
    }
}

/// The rows of the card: label and value, left column then right.
pub fn member_rows(c: &MemberCard) -> [(&'static str, String); 4] {
    let l = look(c.plain);
    [
        ("MEMBER SINCE", since_text(c.since)),
        (l.plays, c.plays.to_string()),
        ("HOURS", hours_text(c.secs)),
        (
            "FAVOURITE GENRE",
            c.genre.clone().map_or("\u{2014}".to_string(), |g| g.to_uppercase()),
        ),
    ]
}

fn fill(px: &mut [u8], w: u32, x0: u32, y0: u32, x1: u32, y1: u32, c: [u8; 3]) {
    for y in y0..y1.min(CARD_H) {
        for x in x0..x1.min(w) {
            let at = ((y * w + x) * 4) as usize;
            if px[at + 3] > 0 {
                px[at..at + 3].copy_from_slice(&c);
            }
        }
    }
}

/// Print `s` with its top left at (`x`, `y`), or its right edge at `x`
/// when `right`; returns how wide it was.
fn print(px: &mut [u8], x: f32, y: f32, s: &str, size: f32, c: [u8; 3], right: bool) -> f32 {
    let Some(font) = text::label_font() else {
        return 0.0;
    };
    let layout = text::fit(font, s, CARD_W as f32 - 2.0 * PAD, 1, size, size * 0.6);
    let measured = layout
        .lines
        .first()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .unwrap_or(0.0);
    let (bw, bh) = ((measured.ceil() as u32).max(1), (size * 1.6).ceil() as u32);
    let cov = text::coverage(bw, bh, &layout);
    let x = if right { x - bw as f32 } else { x };
    for row in 0..bh {
        for col in 0..bw {
            let a = cov[(row * bw + col) as usize] as u32;
            let (dx, dy) = (x as i64 + col as i64, y as i64 + row as i64);
            if a == 0 || dx < 0 || dy < 0 || dx >= CARD_W as i64 || dy >= CARD_H as i64 {
                continue;
            }
            text::blend_pixel(px, CARD_W, dx as u32, dy as u32, a, c);
        }
    }
    measured
}

pub fn member_card_face(c: &MemberCard) -> UndoFace {
    let (w, h) = (CARD_W, CARD_H);
    let l = look(c.plain);
    let mut px = vec![0u8; (w * h * 4) as usize];
    // The card's outline, rounded, softened a pixel at the corners.
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let cx = fx.clamp(RADIUS, w as f32 - RADIUS);
            let cy = fy.clamp(RADIUS, h as f32 - RADIUS);
            let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
            let a = (RADIUS + 0.5 - d).clamp(0.0, 1.0);
            let at = ((y * w + x) * 4) as usize;
            px[at..at + 3].copy_from_slice(&l.body);
            px[at + 3] = (a * 255.0) as u8;
        }
    }
    fill(&mut px, w, 0, 0, w, BAND_H, l.band);
    print(&mut px, PAD, 18.0, l.title, 34.0, l.band_ink, false);
    print(&mut px, w as f32 - PAD, 30.0, l.corner, 17.0, l.band_ink, true);
    let rows = member_rows(c);
    let col = [PAD, w as f32 / 2.0 + 10.0];
    for (n, (label, value)) in rows.iter().enumerate() {
        let (x, y) = (col[n / 2], 100.0 + (n % 2) as f32 * 70.0);
        print(&mut px, x, y, label, 14.0, l.label, false);
        print(&mut px, x, y + 20.0, value, 28.0, l.value, false);
    }
    // A barcode on a white strip, and the build under it.
    let (sx, sy, sw, sh) = (PAD as u32, 248u32, 200u32, 38u32);
    fill(&mut px, w, sx, sy, sx + sw, sy + sh, [0xf6, 0xf4, 0xef]);
    let code: String = c
        .serial
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>()
        .to_uppercase();
    if let Some(elements) = crate::code39(&format!("*{code}*")) {
        // Nine elements a letter, bar and space by turns, wide ones three
        // units across, and a narrow space between letters.
        let units: f32 = elements.iter().map(|wide| if *wide { 3.0 } else { 1.0 }).sum::<f32>()
            + (elements.len() / 9) as f32;
        let unit = (sw - 12) as f32 / units;
        let mut x = (sx + 6) as f32;
        for (i, wide) in elements.iter().enumerate() {
            let width = if *wide { 3.0 } else { 1.0 } * unit;
            if i % 2 == 0 {
                let (x0, x1) = (x.round() as u32, (x + width).round() as u32);
                fill(&mut px, w, x0, sy + 5, x1.max(x0 + 1), sy + sh - 5, [0x14, 0x14, 0x18]);
            }
            x += width;
            if i % 9 == 8 {
                x += unit;
            }
        }
    }
    print(&mut px, w as f32 - PAD, 252.0, &c.serial, 13.0, l.label, true);
    print(&mut px, w as f32 - PAD, 274.0, "A  SMALL PRINT", 13.0, l.label, true);
    // The laminate: a faint sheen across the card.
    for y in 0..h {
        for x in 0..w {
            let t = (x as f32 + y as f32 * 1.6) / (w as f32 + h as f32 * 1.6);
            let sheen = (1.0 - ((t - 0.35) / 0.12).powi(2)).max(0.0) * 26.0;
            let at = ((y * w + x) * 4) as usize;
            for k in 0..3 {
                px[at + k] = (px[at + k] as f32 + sheen).min(255.0) as u8;
            }
        }
    }
    UndoFace { rgba: px, w, h }
}

/// The card in the middle of the screen, a little above centre to leave
/// room for the hints under it.
pub fn draw_member_card(face: Option<TexId>, out: &mut Vec<Draw>) {
    let Some(tex) = face else {
        return;
    };
    out.push(Draw::Tex {
        x: ((OUT_W - CARD_W) / 2) as f32,
        y: ((OUT_H - CARD_H) / 2) as f32 - 20.0,
        w: CARD_W as f32,
        h: CARD_H as f32,
        tex,
        alpha: 1.0,
    });
}
