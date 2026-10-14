//! The night drop: back from a game, its box drops into the shop's
//! returns bin before the shelf shows: through the flap on top, with a
//! clunk and a shake, and the shop fades in behind it.

use shelf_gfx::{Draw, TexId, OUT_H, OUT_W};

use crate::shelf::band;

// The beats, in ms from the start.
const FADE_IN: f32 = 220.0;
const FALL: (f32, f32) = (260.0, 820.0);
/// When the box lands in the bin, for the clunk.
pub const DROP_IMPACT_MS: u64 = 820;
const FADE_OUT: (f32, f32) = (1400.0, 1800.0);
/// How long the night drop lasts, fade out and all.
pub const DROP_MS: u64 = 1800;
/// Where a skip jumps to: the fade out.
pub const DROP_SKIP_TO_MS: u64 = 1400;

const WALL: [f32; 4] = [0.035, 0.05, 0.13, 1.0];
const WALL_LINE: [f32; 4] = [0.06, 0.08, 0.19, 1.0];
const FLOOR: [f32; 4] = [0.02, 0.025, 0.06, 1.0];
const BIN: [f32; 4] = [0.078, 0.282, 0.784, 1.0];
const BIN_DARK: [f32; 4] = [0.05, 0.18, 0.52, 1.0];
const BIN_TOP: [f32; 4] = [0.13, 0.36, 0.86, 1.0];
const YELLOW: [f32; 4] = [0.949, 0.761, 0.0, 1.0];
const MOUTH: [f32; 4] = [0.0, 0.0, 0.02, 1.0];
const LID: [f32; 4] = [0.62, 0.65, 0.72, 1.0];
const LID_EDGE: [f32; 4] = [0.85, 0.87, 0.92, 1.0];

const BIN_W: f32 = 350.0;
/// The bin stands on past the bottom of the screen, so nothing dropped
/// into it shows under it.
const BIN_H: f32 = OUT_H as f32 - BIN_TOP_Y;
const BIN_TOP_Y: f32 = 268.0;
const MOUTH_W: f32 = 286.0;
const LID_H: f32 = 9.0;
/// The tallest the box is drawn.
const BOX_H: f32 = 236.0;

fn span(ms: f32, (from, to): (f32, f32)) -> f32 {
    ((ms - from) / (to - from)).clamp(0.0, 1.0)
}

/// Where the box is `ms` in, for a box `w` by `h`: its top left, or none
/// once it is in the bin.
pub fn box_at(ms: u64, (w, h): (f32, f32)) -> Option<(f32, f32)> {
    let t = span(ms as f32, FALL);
    if t >= 1.0 {
        return None;
    }
    let from = -band() - h - 10.0;
    let to = BIN_TOP_Y + 12.0;
    // Dropped: it gathers speed all the way down.
    Some((OUT_W as f32 / 2.0 - w / 2.0, from + (to - from) * t * t))
}

/// How far the bin is knocked sideways `ms` in, by the box landing.
pub fn bin_shake(ms: u64) -> f32 {
    let since = ms as f32 - DROP_IMPACT_MS as f32;
    if !(0.0..320.0).contains(&since) {
        return 0.0;
    }
    5.0 * (-since / 90.0).exp() * (since / 26.0).sin()
}

/// How far the flap on top is lifted, `ms` in, as the box `h` tall goes in
/// through it: pushed up as the box comes through and slammed shut after,
/// with a bounce.
fn lid_lift(ms: u64, h: f32) -> f32 {
    let t = ms as f32;
    if let Some((_, y)) = box_at(ms, (1.0, h)) {
        return match y + h > BIN_TOP_Y - LID_H {
            true => 16.0,
            false => 0.0,
        };
    }
    let since = t - DROP_IMPACT_MS as f32;
    if (0.0..260.0).contains(&since) {
        // Shut, then a little hop as it settles.
        let hop = (since / 260.0 * std::f32::consts::PI * 2.0).sin().max(0.0);
        return 4.0 * hop * (1.0 - since / 260.0);
    }
    0.0
}

/// The night drop, `ms` in: the shop's wall at night, the returns bin
/// with `sign` on its front, and `front` (the box picture and its size),
/// or a panel of `plain` without one, dropping into it. The whole of it
/// fades in from black and then away, to show the shelf drawn under it.
pub fn draw_night_drop(
    front: Option<(TexId, u32, u32)>,
    plain: [f32; 4],
    sign: Option<(TexId, u32, u32)>,
    ms: u64,
    out: &mut Vec<Draw>,
) {
    if ms >= DROP_MS {
        return;
    }
    let t = ms as f32;
    let alpha = 1.0 - span(t, FADE_OUT);
    let a = |c: [f32; 4]| [c[0], c[1], c[2], c[3] * alpha];
    let top = -band();
    let (sw, sh) = (OUT_W as f32, OUT_H as f32 - top);
    let rect = |x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| Draw::Rect {
        x,
        y,
        w,
        h,
        colour: a(c),
    };
    // The wall, in boards, and the floor.
    out.push(rect(0.0, top, sw, sh, WALL));
    let mut x = 18.0;
    while x < sw {
        out.push(rect(x, top, 2.0, BIN_TOP_Y + BIN_H - top, WALL_LINE));
        x += 72.0;
    }
    let floor = BIN_TOP_Y + BIN_H;
    out.push(rect(0.0, floor, sw, OUT_H as f32 - floor, FLOOR));
    // A lamp over the bin: a soft cone of light down the wall.
    for k in 1..=8 {
        let spread = 40.0 + 26.0 * k as f32;
        out.push(rect(
            OUT_W as f32 / 2.0 - spread,
            top,
            2.0 * spread,
            floor - top,
            [0.45, 0.55, 0.95, 0.018],
        ));
    }
    out.push(rect(OUT_W as f32 / 2.0 - 30.0, top, 60.0, 6.0, [0.9, 0.9, 0.8, 0.9]));
    // The box, falling, behind the bin's front.
    // As tall as it can be and still fit through the mouth.
    let aspect = match front {
        Some((_, w, h)) if w > 0 && h > 0 => w as f32 / h as f32,
        _ => 0.72,
    };
    let bw = (BOX_H * aspect).min(MOUTH_W - 30.0);
    let (bw, bh) = (bw, bw / aspect);
    let shake = bin_shake(ms);
    let bin_x = OUT_W as f32 / 2.0 - BIN_W / 2.0 + shake;
    // The mouth of the bin, under its flap.
    let mouth_x = OUT_W as f32 / 2.0 - MOUTH_W / 2.0 + shake;
    out.push(rect(mouth_x, BIN_TOP_Y - LID_H, MOUTH_W, LID_H + 4.0, MOUTH));
    if let Some((x, y)) = box_at(ms, (bw, bh)) {
        // Tipped a little as it is let go, and straightening as it falls.
        let fall = span(t, FALL);
        let turn = -0.14 * (1.0 - fall).powi(2);
        out.push(match front {
            Some((tex, _, _)) => Draw::Turned {
                x,
                y,
                w: bw,
                h: bh,
                tex,
                alpha,
                turn,
            },
            None => rect(x, y, bw, bh, plain),
        });
    }
    // The bin: its front, a band of yellow, its sign, a darker foot, and a
    // shadow on the floor.
    out.push(rect(bin_x - 6.0, floor - 3.0, BIN_W + 12.0, 8.0, [0.0, 0.0, 0.0, 0.5]));
    out.push(rect(bin_x, BIN_TOP_Y, BIN_W, BIN_H, BIN));
    out.push(rect(bin_x, BIN_TOP_Y, BIN_W, 6.0, BIN_TOP));
    out.push(rect(bin_x, BIN_TOP_Y + BIN_H - 22.0, BIN_W, 22.0, BIN_DARK));
    out.push(rect(bin_x, BIN_TOP_Y + 30.0, BIN_W, 8.0, YELLOW));
    out.push(rect(bin_x, BIN_TOP_Y + 108.0, BIN_W, 8.0, YELLOW));
    // Lit from above and to the left: a bright left edge, a dark right one.
    out.push(rect(bin_x, BIN_TOP_Y, 5.0, BIN_H, [1.0, 1.0, 1.0, 0.16]));
    out.push(rect(bin_x + BIN_W - 8.0, BIN_TOP_Y, 8.0, BIN_H, [0.0, 0.0, 0.1, 0.35]));
    if let Some((tex, w, h)) = sign {
        let fit = ((BIN_W - 60.0) / w as f32).min(50.0 / h as f32).min(1.8);
        let (w, h) = (w as f32 * fit, h as f32 * fit);
        out.push(Draw::Tex {
            x: (bin_x + BIN_W / 2.0 - w / 2.0).round(),
            y: (BIN_TOP_Y + 73.0 - h / 2.0).round(),
            w,
            h,
            tex,
            alpha,
        });
    }
    // The flap over the mouth, lifted as the box goes through.
    let lift = lid_lift(ms, bh);
    let lid_y = BIN_TOP_Y - LID_H - lift;
    out.push(rect(mouth_x - 6.0, lid_y, MOUTH_W + 12.0, LID_H, LID));
    out.push(rect(mouth_x - 6.0, lid_y, MOUTH_W + 12.0, 2.0, LID_EDGE));
    // Out of black at the start.
    let dark = 1.0 - span(t, (0.0, FADE_IN));
    if dark > 0.0 {
        out.push(Draw::Rect {
            x: 0.0,
            y: top,
            w: sw,
            h: sh,
            colour: [0.0, 0.0, 0.0, dark],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_falls_faster_and_faster_and_is_gone_by_the_clunk() {
        let size = (140.0, 200.0);
        let y = |ms| box_at(ms, size).unwrap().1;
        assert!(y(500) > y(300));
        assert!(y(800) - y(700) > y(400) - y(300), "it is not speeding up");
        assert!(box_at(DROP_IMPACT_MS, size).is_none());
    }

    #[test]
    fn the_bin_shakes_only_after_the_clunk_and_stops() {
        assert_eq!(bin_shake(0), 0.0);
        assert_eq!(bin_shake(DROP_IMPACT_MS - 1), 0.0);
        assert!(bin_shake(DROP_IMPACT_MS + 10).abs() > 0.5);
        assert_eq!(bin_shake(DROP_IMPACT_MS + 400), 0.0);
    }

    #[test]
    fn nothing_is_drawn_after_the_drop() {
        let mut out = Vec::new();
        draw_night_drop(None, [1.0; 4], None, DROP_MS, &mut out);
        assert!(out.is_empty());
        draw_night_drop(None, [1.0; 4], None, 500, &mut out);
        assert!(!out.is_empty());
    }
}
