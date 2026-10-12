//! The video store: a second way to show a console's games, as boxes
//! standing face-out on a shop shelf under a hanging aisle sign, in the
//! blue and yellow of the rental shops of the nineties. Everything here is
//! made from rectangles and a few textures drawn in code.

use shelf_gfx::{Draw, TexId, OUT_H, OUT_W};
use shelf_store::{Cart, Platform};

use crate::cart::CartFace;
use crate::details::star_mask;
use crate::hud::Millis;
use crate::plate::{game_name, UndoFace};
use crate::slot_chrome::slot_colours;
use crate::text;

/// The most a box front is drawn: the middle one at full size.
pub const STORE_BOX_W: u32 = 164;
pub const STORE_BOX_H: u32 = 188;
/// How far apart the boxes stand.
pub const STORE_PITCH: f32 = 176.0;
/// Where the boxes' feet are: the top of the shelf board.
pub const STORE_FOOT: f32 = 336.0;
/// How big a box at the side is, against the one in the middle, and how
/// much it is darkened.
pub const STORE_SIDE_SCALE: f32 = 0.8;
pub const STORE_SIDE_SHADE: f32 = 0.42;

/// Where the game's name goes: the sign hangs above it.
pub const STORE_NAME_Y: f32 = 74.0;

const SIGN_Y: f32 = 38.0;
const SIGN_H: f32 = 34.0;
const SIGN_PAD: f32 = 26.0;
const SIGN_MIN_W: f32 = 150.0;
/// The widest the sign's lettering is set, so a long name still hangs
/// on the screen.
const SIGN_MAX_W: f32 = 520.0;
const TAB_MAX_W: f32 = 420.0;
const SIGN_PX: f32 = 22.0;
const SIGN_FACE_H: u32 = 28;
/// How long the sign takes to swing down into place when the aisle changes.
pub const SIGN_IN_MS: Millis = 320;
const SIGN_DROP: f32 = 30.0;
const CEILING_H: f32 = 8.0;

const TAB_PX: f32 = 12.0;
const TAB_H: u32 = 16;

pub const TAG_W: u32 = 60;
pub const TAG_H: u32 = 16;
const TAG_WORDS: [&str; 3] = ["1 NIGHT", "3 NIGHTS", "5 NIGHTS"];
const TAG_STRIPES: [[u8; 3]; 3] = [[0xf2, 0xc2, 0x00], [0xd0, 0x30, 0x30], [0x28, 0x66, 0xe0]];
/// How far under the shelf's rail the price tags hang.
const TAG_DROP: f32 = 6.0;

const YELLOW: [f32; 4] = [0.949, 0.761, 0.0, 1.0];
const SIGN_BLUE: [f32; 4] = [0.078, 0.282, 0.784, 1.0];
const SIGN_INK: [u8; 3] = [0xff, 0xd8, 0x1a];
const CHAIN: [f32; 4] = [0.70, 0.72, 0.80, 1.0];

const RAIL_Y: f32 = STORE_FOOT + 7.0;
const RAIL_H: f32 = 5.0;
const ACCENT_H: f32 = 5.0;
const CABINET_BOTTOM: f32 = 376.0;
const BAND_Y: f32 = 422.0;
const BAY_TOP: f32 = STORE_FOOT - STORE_BOX_H as f32 - 6.0;

/// How wide the box made for a game with no picture is.
pub const PLAIN_W: u32 = 142;

fn px(c: u8) -> f32 {
    c as f32 / 255.0
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale(c: [f32; 3], by: f32) -> [f32; 3] {
    [c[0] * by, c[1] * by, c[2] * by]
}

fn rgb(c: [u8; 3]) -> [f32; 3] {
    [px(c[0]), px(c[1]), px(c[2])]
}

/// The store behind the boxes, a whole screen of it: the ceiling, the dark
/// blue wall, the bay the boxes stand in with its header board, the shelf
/// and the cabinet under it, and the carpet, with a lattice and flecks.
pub fn store_backdrop_face() -> UndoFace {
    let (w, h) = (OUT_W, OUT_H);
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let c = pixel(x, y);
            let at = ((y as u32 * w + x as u32) * 4) as usize;
            for k in 0..3 {
                rgba[at + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
    UndoFace { rgba, w, h }
}

fn pixel(x: i32, y: i32) -> [f32; 3] {
    let (fx, fy) = (x as f32, y as f32);
    let cx = OUT_W as f32 / 2.0;
    // Pools of light under the middle of the shop, dimmer to the sides.
    let light = 1.0 - 0.34 * ((fx - cx).abs() / cx).powf(1.6);
    if fy < CEILING_H {
        // The ceiling, with yellow trim along its lower edge.
        return if fy >= CEILING_H - 2.0 {
            [YELLOW[0], YELLOW[1], YELLOW[2]]
        } else {
            rgb([0x0b, 0x12, 0x38])
        };
    }
    if fy < BAY_TOP - 8.0 {
        // The wall: dark blue, lighter high up and in the middle, in slats.
        let t = fy / STORE_FOOT;
        let base = mix(rgb([0x10, 0x24, 0x6c]), rgb([0x0a, 0x16, 0x4a]), t);
        let slat = if x % 60 == 0 { 1.07 } else { 1.0 };
        return scale(base, light * slat);
    }
    if fy < BAY_TOP {
        // The header board over the bay: lighter blue with a yellow edge.
        if fy >= BAY_TOP - 2.0 {
            return scale([YELLOW[0], YELLOW[1], YELLOW[2]], 0.9);
        }
        let t = (fy - (BAY_TOP - 8.0)) / 6.0;
        let board = mix(rgb([0x24, 0x4a, 0xb4]), rgb([0x1a, 0x38, 0x92]), t);
        return scale(board, 0.6 + 0.4 * light);
    }
    if fy < STORE_FOOT - 3.0 {
        // The bay's back panel: darker, with a pegboard's dots, and the
        // header's shadow along its top.
        let shade = 0.55 + 0.45 * ((fy - BAY_TOP) / 16.0).min(1.0);
        let dot = x % 14 == 7 && y % 14 == 7;
        let c = if dot {
            rgb([0x16, 0x2c, 0x7a])
        } else {
            rgb([0x07, 0x12, 0x3e])
        };
        return scale(c, shade * (0.55 + 0.45 * light));
    }
    if fy < STORE_FOOT + 7.0 {
        // The top of the shelf board, pale and lit from the front.
        let t = (fy - (STORE_FOOT - 3.0)) / 10.0;
        let board = mix(rgb([0xd6, 0xde, 0xf2]), rgb([0x9a, 0xa8, 0xd0]), t);
        return scale(board, 0.7 + 0.3 * light);
    }
    if fy < RAIL_Y + RAIL_H {
        return [YELLOW[0], YELLOW[1], YELLOW[2]];
    }
    if fy < CABINET_BOTTOM {
        // The cabinet's face the price tags hang on.
        let t = (fy - RAIL_Y) / (CABINET_BOTTOM - RAIL_Y);
        let face = mix(rgb([0x12, 0x28, 0x78]), rgb([0x08, 0x12, 0x40]), t);
        return scale(face, 0.6 + 0.4 * light);
    }
    if fy < CABINET_BOTTOM + 6.0 {
        // The skirting.
        return if fy < CABINET_BOTTOM + 1.0 {
            rgb([0x3a, 0x58, 0xb0])
        } else {
            rgb([0x05, 0x09, 0x28])
        };
    }
    carpet(x, y, light)
}

fn carpet(x: i32, y: i32, light: f32) -> [f32; 3] {
    let near = CABINET_BOTTOM + 6.0;
    let depth = ((y as f32 - near) / (OUT_H as f32 - near)).clamp(0.0, 1.0);
    // Darker the nearer, and in the corners, with a shadow under the skirting.
    let lit = (1.0 - 0.38 * depth) * (0.62 + 0.38 * light);
    let under = ((y as f32 - near) / 7.0).clamp(0.0, 1.0);
    let lit = lit * (0.45 + 0.55 * under);
    let mut c = rgb([0x1c, 0x2c, 0x82]);
    // A lattice of diamonds one thread wide, a fleck in the middle of each.
    const CELL: i32 = 28;
    let (a, b) = ((x + y).rem_euclid(CELL), (x - y).rem_euclid(CELL));
    if a == 0 || b == 0 {
        c = mix(c, rgb([0x34, 0x50, 0xb0]), 0.7);
    }
    if (a - CELL / 2).abs() <= 1 && (b - CELL / 2).abs() <= 1 {
        let cell = hash((x + y).div_euclid(CELL), (x - y).div_euclid(CELL));
        let fleck = match cell % 3 {
            0 => rgb([0xc8, 0xa4, 0x20]),
            1 => rgb([0x5a, 0x7e, 0xd8]),
            _ => rgb([0xa0, 0x34, 0x58]),
        };
        c = mix(c, fleck, 0.8);
    }
    // The grain of the pile.
    let grain = 0.94 + 0.12 * ((hash(x, y) % 256) as f32 / 255.0);
    c = scale(c, grain);
    // A yellow line painted on the floor, a little way out.
    if (y as f32) >= 392.0 && (y as f32) < 395.0 {
        c = mix(c, rgb([0xd8, 0xb0, 0x10]), 0.85);
    }
    scale(c, lit)
}

/// The three price tags that go under the boxes: a white label with the
/// number of nights, and a stripe of colour down its left edge.
pub fn price_tag_faces() -> Vec<UndoFace> {
    TAG_WORDS
        .iter()
        .zip(TAG_STRIPES)
        .map(|(word, stripe)| {
            let (w, h) = (TAG_W, TAG_H);
            let mut rgba = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in 0..w {
                    // Clipped corners.
                    let corner = (x < 2 || x >= w - 2) && (y < 2 || y >= h - 2);
                    if corner {
                        continue;
                    }
                    let c = if x < 5 { stripe } else { [0xf6, 0xf5, 0xee] };
                    let at = ((y * w + x) * 4) as usize;
                    rgba[at..at + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
                }
            }
            if let Some(font) = text::label_font() {
                let layout = text::fit(font, word, (w - 10) as f32, 1, 11.0, 8.0);
                let mut inner = vec![0u8; ((w - 5) * h * 4) as usize];
                text::draw_centred(&mut inner, w - 5, h, &layout, [0x0e, 0x18, 0x50]);
                for y in 0..h {
                    for x in 0..w - 5 {
                        let s = ((y * (w - 5) + x) * 4) as usize;
                        if inner[s + 3] == 0 {
                            continue;
                        }
                        text::blend_pixel(
                            &mut rgba,
                            w,
                            x + 5,
                            y,
                            inner[s + 3] as u32,
                            [inner[s], inner[s + 1], inner[s + 2]],
                        );
                    }
                }
            }
            UndoFace { rgba, w, h }
        })
        .collect()
}

/// The box picture at `path`, shrunk to fit the most the store draws a box.
pub fn store_box_face(path: &std::path::Path) -> Option<CartFace> {
    let (rgba, w, h) = crate::art::fit(path, STORE_BOX_W, STORE_BOX_H)?;
    Some(CartFace { rgba, w, h })
}

/// The colour a console's games are marked with along the shelf's rail.
pub fn accent_of(platform: Option<Platform>) -> [f32; 4] {
    let c = slot_colours(platform)[0];
    let lum = 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
    let lift = if lum < 0.35 { 0.4 } else { 0.0 };
    [
        c[0] + (1.0 - c[0]) * lift,
        c[1] + (1.0 - c[1]) * lift,
        c[2] + (1.0 - c[2]) * lift,
        1.0,
    ]
}

/// The box made for a game with no picture: a panel in its console's
/// colour with the console's name along the top and the game's name,
/// large, over the middle.
pub fn plain_box_face(cart: &Cart) -> CartFace {
    let (w, h) = (PLAIN_W, STORE_BOX_H);
    let top = [px(0xff), px(0xff), px(0xff)];
    let panel = accent_of(Some(cart.platform));
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / h as f32;
            let c = mix(
                [panel[0], panel[1], panel[2]],
                scale([panel[0], panel[1], panel[2]], 0.55),
                t,
            );
            let edge = x < 3 || y < 3 || x >= w - 3 || y >= h - 3;
            let c = if edge { mix(c, top, 0.35) } else { c };
            let at = ((y * w + x) * 4) as usize;
            for k in 0..3 {
                rgba[at + k] = (c[k] * 255.0).round() as u8;
            }
        }
    }
    let shade = |rgba: &mut [u8], y0: u32, y1: u32, a: u32| {
        for y in y0..y1 {
            for x in 3..w - 3 {
                text::blend_pixel(rgba, w, x, y, a, [0, 0, 0]);
            }
        }
    };
    shade(&mut rgba, 3, 28, 90);
    shade(&mut rgba, h - 20, h - 3, 70);
    if let Some(font) = text::label_font() {
        let ink = [0xff, 0xff, 0xff];
        let blit = |rgba: &mut [u8],
                    text: &str,
                    y0: u32,
                    bh: u32,
                    max_px: f32,
                    min_px: f32,
                    lines: usize| {
            let layout = text::fit_box(
                font,
                text,
                (w - 20) as f32,
                bh as f32,
                lines,
                max_px,
                min_px,
            );
            let mut strip = vec![0u8; (w * bh * 4) as usize];
            text::draw_centred(&mut strip, w, bh, &layout, ink);
            for y in 0..bh {
                for x in 0..w {
                    let s = ((y * w + x) * 4) as usize;
                    if strip[s + 3] > 0 {
                        text::blend_pixel(
                            rgba,
                            w,
                            x,
                            y0 + y,
                            strip[s + 3] as u32,
                            [strip[s], strip[s + 1], strip[s + 2]],
                        );
                    }
                }
            }
        };
        blit(&mut rgba, cart.platform.name(), 4, 22, 11.0, 7.0, 1);
        blit(
            &mut rgba,
            &game_name(&cart.stem),
            34,
            h - 34 - 22,
            26.0,
            10.0,
            6,
        );
    }
    CartFace { rgba, w, h }
}

/// Words, with a star where the text has a ★, in `ink` on nothing, as a
/// face `h` tall.
fn runs_face(label: &str, size: f32, tracking: f32, ink: [u8; 3], h: u32, max_w: f32) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let upper = label.to_uppercase();
    let mut chars: Vec<char> = upper.chars().collect();
    let set_at = |chars: &[char], size: f32| -> f32 {
        let star = (size * 0.9).round().max(6.0);
        chars
            .iter()
            .map(|c| match c {
                '\u{2605}' => star,
                c => font.metrics(*c, size).advance_width,
            })
            .sum::<f32>()
            + tracking * (chars.len() as f32 - 1.0).max(0.0)
    };
    // A long name is set smaller rather than run off the screen.
    let mut size = size;
    while set_at(&chars, size) > max_w && size > 9.0 {
        size -= 1.0;
    }
    // And one too long even so is cut short.
    if set_at(&chars, size) > max_w {
        while chars.len() > 1 && set_at(&chars, size) > max_w {
            chars.pop();
            chars.pop();
            chars.push('\u{2026}');
        }
    }
    let star = (size * 0.9).round().max(6.0) as u32;
    let advance = |ch: char| match ch {
        '\u{2605}' => star as f32,
        ch => font.metrics(ch, size).advance_width,
    };
    let set = set_at(&chars, size);
    let w = set.ceil() as u32 + 4;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let (ascent, line) = text::line_metrics(font, size);
    let baseline = (h as f32 - line) / 2.0 + ascent;
    let cap = text::cap_height(font, size);
    let mut pen: f32 = 2.0;
    let mask = star_mask(star);
    for ch in chars {
        if ch == '\u{2605}' {
            let centre = baseline - cap / 2.0;
            let (x0, y0) = (
                pen.round() as i32,
                (centre - star as f32 * 0.55).round() as i32,
            );
            for sy in 0..star as i32 {
                for sx in 0..star as i32 {
                    let (dx, dy) = (x0 + sx, y0 + sy);
                    if dx < 0 || dy < 0 || dx >= w as i32 || dy >= h as i32 {
                        continue;
                    }
                    let a = mask[(sy as u32 * star + sx as u32) as usize] as u32;
                    text::blend_pixel(&mut rgba, w, dx as u32, dy as u32, a, ink);
                }
            }
        } else {
            text::draw_line(
                &mut rgba,
                w,
                h,
                pen,
                baseline,
                &ch.to_string(),
                size,
                0.0,
                ink,
                255,
            );
        }
        pen += advance(ch) + tracking;
    }
    UndoFace { rgba, w, h }
}

/// The aisle sign's lettering: yellow capitals, for the plate to go behind.
pub fn sign_face(label: &str) -> UndoFace {
    runs_face(label, SIGN_PX, 2.0, SIGN_INK, SIGN_FACE_H, SIGN_MAX_W)
}

/// The small tab the cart shelf shows its section on: pale small capitals.
pub fn tab_face(label: &str) -> UndoFace {
    runs_face(label, TAB_PX, 2.0, [0xee, 0xee, 0xf4], TAB_H, TAB_MAX_W)
}

/// How tall the tab's face is.
pub const TAB_FACE_H: u32 = TAB_H;

fn rect(x: f32, y: f32, w: f32, h: f32, colour: [f32; 4]) -> Draw {
    Draw::Rect { x, y, w, h, colour }
}

fn with_alpha(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

/// The store, behind everything: the picture of it, or, until that is
/// made, a plain dark blue wall.
pub fn draw_store_backdrop(face: Option<TexId>, accent: [f32; 4], out: &mut Vec<Draw>) {
    match face {
        Some(tex) => out.push(Draw::Tex {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: OUT_H as f32,
            tex,
            alpha: 1.0,
        }),
        None => out.push(rect(
            0.0,
            0.0,
            OUT_W as f32,
            OUT_H as f32,
            [px(0x0a), px(0x16), px(0x4a), 1.0],
        )),
    }
    // The console's colour, as a strip under the rail the tags hang from.
    out.push(rect(0.0, RAIL_Y + RAIL_H, OUT_W as f32, ACCENT_H, accent));
}

/// The dark strip along the foot of the screen, the carpet showing through,
/// that the battery, the console's name and the clock sit on.
pub fn draw_store_band(out: &mut Vec<Draw>) {
    out.push(rect(
        0.0,
        BAND_Y,
        OUT_W as f32,
        OUT_H as f32 - BAND_Y,
        [0.01, 0.02, 0.1, 0.62],
    ));
    out.push(rect(0.0, BAND_Y, OUT_W as f32, 2.0, YELLOW));
}

/// A price tag under the box standing at `centre` (x), by how far off the
/// middle it is (0 to 1).
pub fn draw_price_tag(tag: TexId, centre: f32, off_middle: f32, out: &mut Vec<Draw>) {
    out.push(Draw::Tex {
        x: (centre - TAG_W as f32 / 2.0).round(),
        y: RAIL_Y + RAIL_H + ACCENT_H + TAG_DROP / 2.0,
        w: TAG_W as f32,
        h: TAG_H as f32,
        tex: tag,
        alpha: 1.0 - 0.4 * off_middle.clamp(0.0, 1.0),
    });
}

/// How far down the sign is at `age` since it changed: 1 at rest, less while
/// it is still swinging down.
fn sign_rest(age: Millis) -> f32 {
    let t = (age as f32 / SIGN_IN_MS as f32).min(1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

/// The aisle sign hanging from the ceiling on two chains, `age` ms after
/// its words last changed. `face` is its lettering and size.
pub fn draw_sign(face: Option<(TexId, u32, u32)>, age: Millis, out: &mut Vec<Draw>) {
    let Some((tex, fw, fh)) = face else {
        return;
    };
    let rest = sign_rest(age);
    let top = SIGN_Y - SIGN_DROP * (1.0 - rest);
    let w = (fw as f32 + 2.0 * SIGN_PAD).max(SIGN_MIN_W);
    let x = ((OUT_W as f32 - w) / 2.0).round();
    let a = 0.35 + 0.65 * rest;
    for cx in [x + 16.0, x + w - 18.0] {
        out.push(rect(
            cx,
            CEILING_H,
            2.0,
            top - CEILING_H,
            with_alpha(CHAIN, a),
        ));
    }
    out.push(rect(
        x + 3.0,
        top + 4.0,
        w,
        SIGN_H,
        [0.0, 0.0, 0.0, 0.35 * a],
    ));
    out.push(rect(x, top, w, SIGN_H, with_alpha(YELLOW, a)));
    out.push(rect(
        x + 3.0,
        top + 3.0,
        w - 6.0,
        SIGN_H - 6.0,
        with_alpha(SIGN_BLUE, a),
    ));
    out.push(rect(
        x + 3.0,
        top + 3.0,
        w - 6.0,
        1.0,
        [1.0, 1.0, 1.0, 0.25 * a],
    ));
    out.push(Draw::Tex {
        x: x + (w - fw as f32) / 2.0,
        y: top + (SIGN_H - fh as f32) / 2.0,
        w: fw as f32,
        h: fh as f32,
        tex,
        alpha: a,
    });
}

/// The cart shelf's small section tab, centred, with its top at `y`.
pub fn draw_tab(face: Option<(TexId, u32, u32)>, y: f32, alpha: f32, out: &mut Vec<Draw>) {
    let Some((tex, w, h)) = face else {
        return;
    };
    out.push(Draw::Tex {
        x: ((OUT_W as f32 - w as f32) / 2.0).round(),
        y,
        w: w as f32,
        h: h as f32,
        tex,
        alpha,
    });
}
