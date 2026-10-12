//! The store's strip along the foot of the screen: the console's name on a
//! yellow ticket like the shop's sign, a cassette for the battery, and the
//! time in the red digits of a VCR clock.

use shelf_gfx::{Draw, TexId, OUT_H, OUT_W};
use shelf_power::{Battery, Charge};

use crate::footer::Printed;
use crate::plate::HINT_H;

/// The top of the strip, as `draw_store_band` lays it.
const BAND_Y: f32 = 422.0;
/// The middle of the strip, the line everything on it is centred on.
const MID: f32 = BAND_Y + (OUT_H as f32 - BAND_Y) / 2.0;
const MARGIN: f32 = 22.0;

const YELLOW: [f32; 4] = [0.949, 0.761, 0.0, 1.0];
const YELLOW_DARK: [f32; 4] = [0.72, 0.54, 0.0, 1.0];
const NIGHT: [f32; 4] = [0.02, 0.04, 0.16, 1.0];
const SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 0.4];

const TICKET_H: f32 = 28.0;
const TICKET_PAD: f32 = 26.0;
const TICKET_MIN_W: f32 = 150.0;
/// The bites out of each end of the ticket, where it was torn off the roll.
const TOOTH: f32 = 4.0;
/// How far in from the left end the stub's perforation runs.
const STUB: f32 = 15.0;

/// The ticket's lettering: the shop's blue on its yellow.
pub const TICKET_INK: [u8; 3] = [0x10, 0x2a, 0x8c];

const DIGIT_W: f32 = 13.0;
const DIGIT_H: f32 = 24.0;
const SEG: f32 = 3.0;
const DIGIT_GAP: f32 = 4.0;
const COLON_W: f32 = 12.0;
const PANEL_PAD: f32 = 7.0;
const LIT: [f32; 4] = [1.0, 0.16, 0.12, 1.0];
const GLOW: [f32; 4] = [1.0, 0.1, 0.05, 0.18];
const UNLIT: [f32; 4] = [0.35, 0.04, 0.04, 0.22];

const TAPE_W: f32 = 44.0;
const TAPE_H: f32 = 28.0;

fn rect(x: f32, y: f32, w: f32, h: f32, colour: [f32; 4]) -> Draw {
    Draw::Rect { x, y, w, h, colour }
}

fn with_alpha(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

/// Everything the store prints along its foot. `ticket` is the console's
/// name set in `TICKET_INK`, faded by `alpha` as the name on the cart
/// shelf's slot is; `wall_secs` is the time to show, if the clock is known.
pub fn draw_store_footer(
    ticket: Option<(TexId, u32, u32)>,
    alpha: f32,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    wall_secs: Option<i64>,
    out: &mut Vec<Draw>,
) {
    if let Some(face) = ticket.filter(|_| alpha > 0.0) {
        draw_ticket(face, alpha, out);
    }
    if let Some(b) = battery {
        draw_cassette(b, percent, bolt, out);
    }
    if let Some(secs) = wall_secs {
        draw_vcr_clock(secs, out);
    }
}

fn draw_ticket((tex, fw, fh): (TexId, u32, u32), a: f32, out: &mut Vec<Draw>) {
    let w = (fw as f32 + 2.0 * TICKET_PAD + STUB).max(TICKET_MIN_W).round();
    let x = ((OUT_W as f32 - w) / 2.0).round();
    let y = (MID - TICKET_H / 2.0).round();
    out.push(rect(x + 2.0, y + 3.0, w, TICKET_H, with_alpha(SHADOW, a)));
    out.push(rect(x, y, w, TICKET_H, with_alpha(YELLOW, a)));
    // Torn ends: small bites out of each side, in the strip's colour.
    let teeth = (TICKET_H / (2.0 * TOOTH)) as i32;
    for i in 0..teeth {
        let ty = y + TOOTH / 2.0 + i as f32 * 2.0 * TOOTH;
        out.push(rect(x, ty, TOOTH / 2.0, TOOTH, with_alpha(NIGHT, a)));
        out.push(rect(x + w - TOOTH / 2.0, ty, TOOTH / 2.0, TOOTH, with_alpha(NIGHT, a)));
    }
    // The stub's perforation, a dotted line down the ticket.
    let mut py = y + 3.0;
    while py < y + TICKET_H - 4.0 {
        out.push(rect(x + STUB, py, 1.5, 2.0, with_alpha(YELLOW_DARK, a)));
        py += 4.0;
    }
    let left = x + STUB;
    out.push(Draw::Tex {
        x: (left + (x + w - left - fw as f32) / 2.0).round(),
        y: (y + (TICKET_H - fh as f32) / 2.0).round(),
        w: fw as f32,
        h: fh as f32,
        tex,
        alpha: a,
    });
}

/// Which of a digit's seven bars are lit: top, top right, bottom right,
/// bottom, bottom left, top left, middle.
fn segments(d: u8) -> [bool; 7] {
    match d {
        0 => [true, true, true, true, true, true, false],
        1 => [false, true, true, false, false, false, false],
        2 => [true, true, false, true, true, false, true],
        3 => [true, true, true, true, false, false, true],
        4 => [false, true, true, false, false, true, true],
        5 => [true, false, true, true, false, true, true],
        6 => [true, false, true, true, true, true, true],
        7 => [true, true, true, false, false, false, false],
        8 => [true; 7],
        _ => [true, true, true, true, false, true, true],
    }
}

fn draw_digit(x: f32, y: f32, d: u8, out: &mut Vec<Draw>) {
    let half = DIGIT_H / 2.0;
    let bars = [
        (x + SEG, y, DIGIT_W - 2.0 * SEG, SEG),
        (x + DIGIT_W - SEG, y + SEG, SEG, half - 1.5 * SEG),
        (x + DIGIT_W - SEG, y + half + SEG / 2.0, SEG, half - 1.5 * SEG),
        (x + SEG, y + DIGIT_H - SEG, DIGIT_W - 2.0 * SEG, SEG),
        (x, y + half + SEG / 2.0, SEG, half - 1.5 * SEG),
        (x, y + SEG, SEG, half - 1.5 * SEG),
        (x + SEG, y + half - SEG / 2.0, DIGIT_W - 2.0 * SEG, SEG),
    ];
    for (on, (bx, by, bw, bh)) in segments(d).into_iter().zip(bars) {
        if on {
            out.push(rect(bx - 1.0, by - 1.0, bw + 2.0, bh + 2.0, GLOW));
            out.push(rect(bx, by, bw, bh, LIT));
        } else {
            out.push(rect(bx, by, bw, bh, UNLIT));
        }
    }
}

/// The time as a VCR shows it: red bars on a black window.
pub fn draw_vcr_clock(secs: i64, out: &mut Vec<Draw>) {
    let rem = secs.rem_euclid(86_400);
    let (h, m) = ((rem / 3600) as u8, (rem / 60 % 60) as u8);
    let digits = [h / 10, h % 10, m / 10, m % 10];
    let inner = 4.0 * DIGIT_W + 2.0 * DIGIT_GAP + COLON_W;
    let w = inner + 2.0 * PANEL_PAD;
    let ph = DIGIT_H + 2.0 * 4.0;
    let x = OUT_W as f32 - MARGIN - w;
    let y = (MID - ph / 2.0).round();
    out.push(rect(x - 1.0, y - 1.0, w + 2.0, ph + 2.0, [0.3, 0.32, 0.4, 0.6]));
    out.push(rect(x, y, w, ph, [0.02, 0.02, 0.03, 1.0]));
    let dy = y + 4.0;
    let mut dx = x + PANEL_PAD;
    for (n, d) in digits.into_iter().enumerate() {
        draw_digit(dx, dy, d, out);
        dx += DIGIT_W;
        match n {
            1 => {
                let cx = dx + (COLON_W - SEG) / 2.0;
                for cy in [dy + DIGIT_H * 0.3, dy + DIGIT_H * 0.7 - SEG] {
                    out.push(rect(cx - 1.0, cy - 1.0, SEG + 2.0, SEG + 2.0, GLOW));
                    out.push(rect(cx, cy, SEG, SEG, LIT));
                }
                dx += COLON_W;
            }
            3 => {}
            _ => dx += DIGIT_GAP,
        }
    }
}

/// The battery as a cassette: tape wound from one reel to the other as it
/// runs down, its percentage printed beside it.
fn draw_cassette(b: Battery, percent: Printed, bolt: Option<TexId>, out: &mut Vec<Draw>) {
    let mut x = MARGIN;
    let y = (MID - TAPE_H / 2.0).round();
    if let (Charge::Charging, Some(tex)) = (b.charge, bolt) {
        out.push(Draw::Tex {
            x,
            y: MID - 7.0,
            w: 14.0,
            h: 14.0,
            tex,
            alpha: 1.0,
        });
        x += 19.0;
    }
    let level = f32::from(b.percent.min(100)) / 100.0;
    // The shell, its label and the window over the reels.
    out.push(rect(x + 2.0, y + 2.0, TAPE_W, TAPE_H, SHADOW));
    out.push(rect(x, y, TAPE_W, TAPE_H, [0.13, 0.13, 0.15, 1.0]));
    out.push(rect(x + 3.0, y + 3.0, TAPE_W - 6.0, TAPE_H - 9.0, [0.93, 0.9, 0.82, 1.0]));
    out.push(rect(x + 3.0, y + 3.0, TAPE_W - 6.0, 3.0, with_alpha(YELLOW, 1.0)));
    let (wx, wy, ww, wh) = (x + 8.0, y + 8.0, TAPE_W - 16.0, 9.0);
    out.push(rect(wx, wy, ww, wh, [0.05, 0.04, 0.04, 1.0]));
    // Tape on the left reel for what is left, on the right for what is used.
    let most = wh - 2.0;
    let tape = [0.32, 0.18, 0.1, 1.0];
    let (lh, rh) = (2.0 + most * level, 2.0 + most * (1.0 - level));
    let (lh, rh) = (lh.min(most), rh.min(most));
    let reel = 8.0;
    out.push(rect(wx + 2.0, wy + (wh - lh) / 2.0, reel, lh, tape));
    out.push(rect(wx + ww - 2.0 - reel, wy + (wh - rh) / 2.0, reel, rh, tape));
    for rx in [wx + 2.0 + reel / 2.0 - 1.5, wx + ww - 2.0 - reel / 2.0 - 1.5] {
        out.push(rect(rx, wy + wh / 2.0 - 1.5, 3.0, 3.0, [0.95, 0.95, 0.95, 1.0]));
    }
    // A low battery shows the tape window's edge in red.
    if b.percent <= 15 {
        out.push(rect(wx, wy + wh, ww, 1.5, LIT));
    }
    if percent.w > 0 {
        crate::footer::draw_printed(
            x + TAPE_W + 9.0,
            MID - HINT_H as f32 / 2.0,
            percent,
            out,
        );
    }
}
