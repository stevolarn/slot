//! The store's strip along the foot of the screen: the console's name on a
//! yellow ticket like the shop's sign, an AA cell for the battery, and the
//! time in the red digits of a VCR clock.

use shelf_gfx::{Draw, TexId, OUT_W};
use shelf_power::{Battery, Charge};

use crate::footer::Printed;
use crate::plate::HINT_H;

/// The top of the strip, as `draw_store_band` lays it.
const BAND_Y: f32 = 422.0;
/// The yellow line under the strip's top edge, lower down, painted on the
/// carpet: everything on the strip is centred between the two.
const LOWER_LINE: f32 = 459.0;
const MID: f32 = (BAND_Y + LOWER_LINE) / 2.0;
const MARGIN: f32 = 22.0;

const YELLOW: [f32; 4] = [0.949, 0.761, 0.0, 1.0];
const YELLOW_DARK: [f32; 4] = [0.72, 0.54, 0.0, 1.0];
const NIGHT: [f32; 4] = [0.02, 0.04, 0.16, 1.0];
const SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 0.4];

const TICKET_H: f32 = 26.0;
const TICKET_PAD: f32 = 26.0;
const TICKET_MIN_W: f32 = 150.0;
/// The bites out of each end of the ticket, where it was torn off the roll.
const TOOTH: f32 = 4.0;
/// How far in from the left end the stub's perforation runs.
const STUB: f32 = 15.0;

/// The ticket's lettering: the shop's blue on its yellow.
pub const TICKET_INK: [u8; 3] = [0x10, 0x2a, 0x8c];

const DIGIT_W: f32 = 13.0;
const DIGIT_H: f32 = 22.0;
const SEG: f32 = 3.0;
const DIGIT_GAP: f32 = 4.0;
const COLON_W: f32 = 12.0;
const PANEL_PAD: f32 = 7.0;
const LIT: [f32; 4] = [1.0, 0.16, 0.12, 1.0];
const GLOW: [f32; 4] = [1.0, 0.1, 0.05, 0.18];
const UNLIT: [f32; 4] = [0.35, 0.04, 0.04, 0.22];

const CELL_W: f32 = 50.0;
const CELL_H: f32 = 22.0;
const CAP_W: f32 = 12.0;
const NUB_W: f32 = 3.0;
const NUB_H: f32 = 9.0;
const BAR_GAP: f32 = 2.0;
const COPPER: [f32; 4] = [0.78, 0.45, 0.2, 1.0];

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
    console_up: bool,
    out: &mut Vec<Draw>,
) {
    // With the console come up over the strip, they sit along its plate.
    let mid = match console_up {
        true => crate::footer::FOOTER_Y + HINT_H as f32 / 2.0,
        false => MID,
    };
    if let Some(face) = ticket.filter(|_| alpha > 0.0) {
        draw_ticket(face, alpha, mid, out);
    }
    if let Some(b) = battery {
        draw_cell(b, percent, bolt, mid, out);
    }
    if let Some(secs) = wall_secs {
        draw_vcr_clock(secs, mid, out);
    }
}

fn draw_ticket((tex, fw, fh): (TexId, u32, u32), a: f32, mid: f32, out: &mut Vec<Draw>) {
    let w = (fw as f32 + 2.0 * TICKET_PAD + STUB).max(TICKET_MIN_W).round();
    let x = ((OUT_W as f32 - w) / 2.0).round();
    let y = (mid - TICKET_H / 2.0).round();
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
fn draw_vcr_clock(secs: i64, mid: f32, out: &mut Vec<Draw>) {
    let rem = secs.rem_euclid(86_400);
    let (h, m) = ((rem / 3600) as u8, (rem / 60 % 60) as u8);
    let digits = [h / 10, h % 10, m / 10, m % 10];
    let inner = 4.0 * DIGIT_W + 2.0 * DIGIT_GAP + COLON_W;
    let w = inner + 2.0 * PANEL_PAD;
    let ph = DIGIT_H + 2.0 * 3.0;
    let x = OUT_W as f32 - MARGIN - w;
    let y = (mid - ph / 2.0).round();
    out.push(rect(x - 1.0, y - 1.0, w + 2.0, ph + 2.0, [0.3, 0.32, 0.4, 0.6]));
    out.push(rect(x, y, w, ph, [0.02, 0.02, 0.03, 1.0]));
    let dy = y + 3.0;
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

/// The battery as an old copper-topped AA cell lying on its side, its
/// charge in four chunky bars, its percentage printed beside it.
fn draw_cell(b: Battery, percent: Printed, bolt: Option<TexId>, mid: f32, out: &mut Vec<Draw>) {
    let mut x = MARGIN;
    let y = (mid - CELL_H / 2.0).round();
    if let (Charge::Charging, Some(tex)) = (b.charge, bolt) {
        out.push(Draw::Tex {
            x,
            y: mid - 7.0,
            w: 14.0,
            h: 14.0,
            tex,
            alpha: 1.0,
        });
        x += 19.0;
    }
    let body = CELL_W - CAP_W;
    out.push(rect(x + 2.0, y + 2.0, CELL_W + NUB_W, CELL_H, SHADOW));
    // The black body, the copper end and the + nub.
    out.push(rect(x, y, body, CELL_H, [0.08, 0.08, 0.09, 1.0]));
    out.push(rect(x + body, y, CAP_W, CELL_H, COPPER));
    out.push(rect(x + body, y, 1.0, CELL_H, [0.0, 0.0, 0.0, 0.5]));
    out.push(rect(
        x + CELL_W,
        y + (CELL_H - NUB_H) / 2.0,
        NUB_W,
        NUB_H,
        [0.78, 0.78, 0.8, 1.0],
    ));
    // The round of the can: a shine along the top, a shade along the foot.
    out.push(rect(x, y + 2.0, CELL_W, 2.0, [1.0, 1.0, 1.0, 0.22]));
    out.push(rect(x, y + CELL_H - 3.0, CELL_W, 3.0, [0.0, 0.0, 0.0, 0.3]));
    let lit = match b.percent {
        0..=15 => LIT,
        16..=40 => YELLOW,
        _ => [0.35, 0.85, 0.3, 1.0],
    };
    let bars = (f32::from(b.percent.min(100)) / 25.0).ceil() as usize;
    let (bx, by, bh) = (x + 4.0, y + 5.0, CELL_H - 10.0);
    let bw = (body - 8.0 - 3.0 * BAR_GAP) / 4.0;
    for n in 0..4 {
        let colour = match n < bars {
            true => lit,
            false => [1.0, 1.0, 1.0, 0.1],
        };
        out.push(rect(bx + n as f32 * (bw + BAR_GAP), by, bw, bh, colour));
    }
    if percent.w > 0 {
        crate::footer::draw_printed(
            x + CELL_W + NUB_W + 9.0,
            mid - HINT_H as f32 / 2.0,
            percent,
            out,
        );
    }
}
