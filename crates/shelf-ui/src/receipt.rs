//! The return receipt: a till slip that prints out of a slot at the foot
//! of the screen when a game is handed back, and the same slips on the
//! spike, newest on top. The video store's says when it was RENTED and
//! RETURNED; the original theme's is a plain grey session slip.

use shelf_store::civil_from_days;

use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::plate::UndoFace;
use crate::text;

pub const SLIP_W: u32 = 280;
pub const SLIP_H: u32 = 396;
const PAD: f32 = 22.0;
/// How deep the teeth of the torn edge at the foot of the slip are.
const TEETH: u32 = 8;
/// The top of the till's slot the slip comes out of.
pub const SLOT_Y: f32 = OUT_H as f32 - 22.0;
const SLOT_LIP: f32 = 7.0;
/// How long after the shelf shows the till starts printing.
pub const SLIP_DELAY_MS: u64 = 500;
/// How long the slip takes to print, and to tear off and go.
pub const SLIP_PRINT_MS: u64 = 900;
pub const SLIP_TEAR_MS: u64 = 320;
/// How long the slip stays out before it tears itself off.
pub const SLIP_STAY_MS: u64 = 7000;
const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// What a receipt says. Times are unix seconds on the local wall clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptSlip {
    /// The original theme's plain session slip, not the shop's.
    pub plain: bool,
    pub console: String,
    pub title: String,
    pub rented: i64,
    pub returned: i64,
    pub secs: u64,
    pub total: u64,
    /// Which receipt this is, counting from the first.
    pub number: u32,
}

struct Look {
    paper: [u8; 3],
    ink: [u8; 3],
    faint: [u8; 3],
    title: &'static str,
    kind: &'static str,
    out: &'static str,
    back: &'static str,
    this: &'static str,
    thanks: &'static str,
}

fn look(plain: bool) -> Look {
    match plain {
        false => Look {
            paper: [0xf7, 0xf4, 0xea],
            ink: [0x2e, 0x2c, 0x34],
            faint: [0x8a, 0x86, 0x8e],
            title: "SHELF VIDEO",
            kind: "RETURN RECEIPT",
            out: "RENTED",
            back: "RETURNED",
            this: "THIS RENTAL",
            thanks: "THANK YOU FOR RENTING",
        },
        true => Look {
            paper: [0xdf, 0xdf, 0xdd],
            ink: [0x2a, 0x2a, 0x2e],
            faint: [0x86, 0x86, 0x8c],
            title: "SHELF",
            kind: "SESSION SLIP",
            out: "STARTED",
            back: "FINISHED",
            this: "PLAYED",
            thanks: "SEE YOU NEXT TIME",
        },
    }
}

fn day_month(secs: i64) -> (i64, &'static str, i64) {
    let (year, month, day) = civil_from_days(secs.div_euclid(86_400));
    let month = MONTHS.get((month - 1) as usize).copied().unwrap_or_default();
    (day, month, year)
}

/// "13 OCT 09:12".
pub fn slip_time_text(secs: i64) -> String {
    let (day, month, _) = day_month(secs);
    let mins = secs.rem_euclid(86_400) / 60;
    format!("{day} {month} {:02}:{:02}", mins / 60, mins % 60)
}

/// "16 OCT 2026".
pub fn slip_date_text(secs: i64) -> String {
    let (day, month, year) = day_month(secs);
    format!("{day} {month} {year}")
}

/// "1H 12M", or "UNDER 1M".
pub fn slip_span_text(secs: u64) -> String {
    crate::play_time_text(secs).unwrap_or_else(|| "UNDER 1M".to_string())
}

/// The rows under the game: label and value, in groups between rules.
pub fn slip_rows(s: &ReceiptSlip) -> Vec<Vec<(&'static str, String)>> {
    let l = look(s.plain);
    vec![
        vec![
            (l.out, slip_time_text(s.rented)),
            (l.back, slip_time_text(s.returned)),
        ],
        vec![
            (l.this, slip_span_text(s.secs)),
            ("TOTAL TIME", slip_span_text(s.total)),
        ],
    ]
}

struct Paper {
    px: Vec<u8>,
}

impl Paper {
    fn width(text: &str, size: f32) -> f32 {
        let Some(font) = text::label_font() else {
            return 0.0;
        };
        text::line_width(font, text, size, text::fit(font, text, 1e6, 1, size, size).tracking)
    }

    /// `s` with its baseline at `y`, its left end at `x`, or its right end
    /// there when `right`.
    fn print(&mut self, x: f32, y: f32, s: &str, size: f32, c: [u8; 3], right: bool) {
        let Some(font) = text::label_font() else { return };
        let layout = text::fit(font, s, SLIP_W as f32 - 2.0 * PAD, 1, size, size * 0.6);
        let Some(line) = layout.lines.first() else { return };
        let w = text::line_width(font, line, layout.px, layout.tracking);
        let x = if right { x - w } else { x };
        text::draw_line(
            &mut self.px, SLIP_W, SLIP_H, x, y, line, layout.px, layout.tracking, c, 255,
        );
    }

    fn centred(&mut self, y: f32, s: &str, size: f32, c: [u8; 3]) {
        let w = Self::width(&s.to_uppercase(), size).min(SLIP_W as f32 - 2.0 * PAD);
        self.print((SLIP_W as f32 - w) / 2.0, y, s, size, c, false);
    }

    fn fill(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, c: [u8; 3]) {
        for y in y0..y1.min(SLIP_H) {
            for x in x0..x1.min(SLIP_W) {
                let at = ((y * SLIP_W + x) * 4) as usize;
                if self.px[at + 3] > 0 {
                    self.px[at..at + 3].copy_from_slice(&c);
                }
            }
        }
    }

    /// A dashed rule across the slip.
    fn rule(&mut self, y: u32, c: [u8; 3]) {
        let mut x = PAD as u32;
        while x + 4 <= SLIP_W - PAD as u32 {
            self.fill(x, y, x + 4, y + 2, c);
            x += 8;
        }
    }
}

/// The paper, with a torn edge at its foot.
fn blank(paper: [u8; 3]) -> Paper {
    let (w, h) = (SLIP_W, SLIP_H);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let tooth = (TEETH * 2) as f32;
    for y in 0..h {
        for x in 0..w {
            // The teeth: a zigzag TEETH deep along the bottom.
            let phase = (x as f32 % tooth) / tooth;
            let depth = (1.0 - (phase * 2.0 - 1.0).abs()) * TEETH as f32;
            let edge = h as f32 - TEETH as f32 + depth;
            let a = (edge - y as f32).clamp(0.0, 1.0);
            let at = ((y * w + x) * 4) as usize;
            px[at..at + 3].copy_from_slice(&paper);
            px[at + 3] = (a * 255.0) as u8;
        }
    }
    Paper { px }
}

/// The curl: thermal paper rolls back at the top, so it darkens there.
fn curl(p: &mut Paper) {
    for y in 0..36u32 {
        let k = (1.0 - y as f32 / 36.0).powi(2) * 0.16;
        for x in 0..SLIP_W {
            let at = ((y * SLIP_W + x) * 4) as usize;
            for c in 0..3 {
                p.px[at + c] = (p.px[at + c] as f32 * (1.0 - k)) as u8;
            }
        }
    }
}

fn barcode(p: &mut Paper, text: &str, y: u32, ink: [u8; 3]) {
    let Some(elements) = crate::code39(&format!("*{text}*")) else {
        return;
    };
    let units: f32 = elements.iter().map(|wide| if *wide { 3.0 } else { 1.0 }).sum::<f32>()
        + (elements.len() / 9) as f32;
    let width = 180.0f32;
    let unit = width / units;
    let mut x = (SLIP_W as f32 - width) / 2.0;
    for (i, wide) in elements.iter().enumerate() {
        let w = if *wide { 3.0 } else { 1.0 } * unit;
        if i % 2 == 0 {
            let (x0, x1) = (x.round() as u32, (x + w).round() as u32);
            p.fill(x0, y, x1.max(x0 + 1), y + 30, ink);
        }
        x += w;
        if i % 9 == 8 {
            x += unit;
        }
    }
}

/// The receipt, or with `None` the slip on an empty spike.
pub fn receipt_face(slip: Option<&ReceiptSlip>, plain: bool) -> UndoFace {
    let l = look(slip.map_or(plain, |s| s.plain));
    let mut p = blank(l.paper);
    p.centred(48.0, l.title, 30.0, l.ink);
    p.centred(70.0, l.kind, 13.0, l.faint);
    let right = SLIP_W as f32 - PAD;
    let Some(s) = slip else {
        p.rule(96, l.faint);
        p.centred(200.0, "NOTHING ON", 20.0, l.ink);
        p.centred(226.0, "THE SPIKE YET", 20.0, l.ink);
        p.centred(270.0, "PLAY A GAME TO", 13.0, l.faint);
        p.centred(290.0, "GET A RECEIPT", 13.0, l.faint);
        curl(&mut p);
        return UndoFace { rgba: p.px, w: SLIP_W, h: SLIP_H };
    };
    p.print(PAD, 92.0, &format!("NO. {:04}", s.number), 12.0, l.faint, false);
    p.print(right, 92.0, &slip_date_text(s.returned), 12.0, l.faint, true);
    p.rule(102, l.faint);
    p.print(PAD, 126.0, &s.console, 13.0, l.faint, false);
    if let Some(font) = text::label_font() {
        // One line if it will go at a readable size, else two.
        let width = SLIP_W as f32 - 2.0 * PAD;
        let one = text::fit(font, &s.title, width, 1, 22.0, 15.0);
        let layout = match one.lines.first() == Some(&s.title.to_uppercase()) {
            true => one,
            false => text::fit(font, &s.title, width, 2, 20.0, 13.0),
        };
        let (_, pitch) = text::line_metrics(font, layout.px);
        for (n, line) in layout.lines.iter().enumerate() {
            let y = 152.0 + n as f32 * pitch.min(26.0);
            text::draw_line(
                &mut p.px, SLIP_W, SLIP_H, PAD, y, line, layout.px, layout.tracking, l.ink, 255,
            );
        }
    }
    let mut y = 186.0;
    for group in slip_rows(s) {
        p.rule(y as u32, l.faint);
        y += 22.0;
        for (label, value) in group {
            p.print(PAD, y, label, 13.0, l.faint, false);
            p.print(right, y, &value, 16.0, l.ink, true);
            y += 20.0;
        }
        y -= 6.0;
    }
    let thanks_y = 316.0;
    p.centred(thanks_y, l.thanks, 13.0, l.ink);
    let code = format!("R{:04}", s.number);
    barcode(&mut p, &code, thanks_y as u32 + 12, l.ink);
    p.centred(thanks_y + 60.0, &code, 11.0, l.faint);
    curl(&mut p);
    UndoFace { rgba: p.px, w: SLIP_W, h: SLIP_H }
}

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// The slip coming out of the till: `t` from 0 to 1 as it prints, then
/// `tear` from 0 to 1 as it is torn off and lifted away.
pub fn draw_receipt_slip(face: Option<TexId>, t: f32, tear: f32, out: &mut Vec<Draw>) {
    let gone = ease_out(tear);
    let shown = ease_out(t);
    // The shop dims behind it.
    out.push(Draw::Rect {
        x: 0.0,
        y: -crate::shelf::band(),
        w: OUT_W as f32,
        h: OUT_H as f32 + crate::shelf::band(),
        colour: [0.0, 0.0, 0.0, 0.5 * t.clamp(0.0, 1.0) * (1.0 - gone)],
    });
    let x = ((OUT_W - SLIP_W) / 2) as f32;
    let sw = SLIP_W as f32;
    if let Some(tex) = face {
        if tear > 0.0 {
            // Torn off: the whole slip, its torn edge and all, lifts away.
            let top = SLOT_Y + SLOT_LIP - SLIP_H as f32 - 60.0 * gone;
            out.push(Draw::Tex {
                x,
                y: top,
                w: sw,
                h: SLIP_H as f32,
                tex,
                alpha: 1.0 - gone,
            });
        } else {
            // Printing: as much of the top as has come out of the slot.
            let out_h = (SLIP_H - TEETH) as f32 * shown;
            if out_h >= 1.0 {
                out.push(Draw::Part {
                    x,
                    y: SLOT_Y + SLOT_LIP - out_h,
                    w: sw,
                    h: out_h,
                    tex,
                    alpha: 1.0,
                    u0: 0.0,
                    v0: 0.0,
                    u1: 1.0,
                    v1: out_h / SLIP_H as f32,
                });
            }
        }
    }
    // The till's slot: a dark mouth with a lighter lip.
    let fade = (1.0 - gone) * (t * 4.0).clamp(0.0, 1.0);
    let (slot_x, slot_w) = (x - 24.0, sw + 48.0);
    out.push(Draw::Rect {
        x: slot_x,
        y: SLOT_Y,
        w: slot_w,
        h: OUT_H as f32 - SLOT_Y,
        colour: [0.16, 0.16, 0.18, fade],
    });
    out.push(Draw::Rect {
        x: slot_x + 10.0,
        y: SLOT_Y + SLOT_LIP,
        w: slot_w - 20.0,
        h: 4.0,
        colour: [0.02, 0.02, 0.03, fade],
    });
    out.push(Draw::Rect {
        x: slot_x,
        y: SLOT_Y,
        w: slot_w,
        h: 2.0,
        colour: [0.42, 0.42, 0.46, fade],
    });
}

/// How long a receipt takes to come off the spike, or go back on it.
pub const SPIKE_TURN_MS: u64 = 280;

/// A receipt going: `prev`, the one that was showing, is pulled up off the
/// spike to show an older one (`older`), or a newer one comes down onto
/// it; `t` runs from 0 to 1.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SpikeTurn {
    pub prev: TexId,
    pub older: bool,
    pub t: f32,
}

/// The spike: a receipt stuck on a steel spike, older ones under it.
/// `under` is how many older receipts are beneath the one shown.
pub fn draw_receipt_spike(
    face: Option<TexId>,
    under: usize,
    turn: Option<SpikeTurn>,
    out: &mut Vec<Draw>,
) {
    let x = ((OUT_W - SLIP_W) / 2) as f32;
    let top = 16.0;
    let sw = SLIP_W as f32;
    let spike_x = OUT_W as f32 / 2.0;
    // How far up a slip is clear of the spike.
    let away = SLIP_H as f32 + 80.0;
    // The spike's base, and its rod behind the paper.
    out.push(Draw::Rect {
        x: spike_x - 70.0,
        y: OUT_H as f32 - 22.0,
        w: 140.0,
        h: 22.0,
        colour: [0.2, 0.2, 0.22, 1.0],
    });
    out.push(Draw::Rect {
        x: spike_x - 2.0,
        y: top - 30.0,
        w: 4.0,
        h: OUT_H as f32 - top + 8.0,
        colour: [0.62, 0.63, 0.66, 1.0],
    });
    // The edges of the older slips, a little askew under it.
    for n in (0..under.min(3)).rev() {
        let k = (n + 1) as f32;
        let dx = if n % 2 == 0 { 7.0 } else { -6.0 } * k;
        out.push(Draw::Rect {
            x: x + dx,
            y: top + 6.0 * k,
            w: sw,
            h: SLIP_H as f32 - TEETH as f32,
            colour: [0.78 - 0.06 * k, 0.77 - 0.06 * k, 0.74 - 0.06 * k, 1.0],
        });
    }
    let slip = |tex: TexId, y: f32, alpha: f32, out: &mut Vec<Draw>| {
        out.push(Draw::Tex {
            x,
            y,
            w: sw,
            h: SLIP_H as f32,
            tex,
            alpha,
        });
    };
    let hole = |y: f32, out: &mut Vec<Draw>| {
        out.push(Draw::Rect {
            x: spike_x - 4.0,
            y: y + 6.0,
            w: 8.0,
            h: 8.0,
            colour: [0.25, 0.25, 0.28, 1.0],
        });
    };
    let turn = turn.filter(|t| t.t < 1.0);
    match (turn, face) {
        // Pulled up off the spike, the older one is left showing.
        (Some(SpikeTurn { prev, older: true, t }), Some(tex)) => {
            slip(tex, top, 1.0, out);
            hole(top, out);
            let k = t.clamp(0.0, 1.0).powi(2);
            slip(prev, top - away * k, 1.0, out);
        }
        // A newer one comes back down onto the spike.
        (Some(SpikeTurn { prev, older: false, t }), Some(tex)) => {
            slip(prev, top, 1.0, out);
            hole(top, out);
            let k = 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3);
            let y = top - away * (1.0 - k);
            slip(tex, y, 1.0, out);
            hole(y, out);
        }
        (_, Some(tex)) => {
            slip(tex, top, 1.0, out);
            hole(top, out);
        }
        (_, None) => {}
    }
    // The point above the paper.
    out.push(Draw::Rect {
        x: spike_x - 2.0,
        y: top - 30.0,
        w: 4.0,
        h: 38.0,
        colour: [0.7, 0.71, 0.74, 1.0],
    });
}
