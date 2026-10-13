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

/// The most a box front is drawn by the old small store faces, still made
/// for a game with no picture.
pub const STORE_BOX_W: u32 = 164;
pub const STORE_BOX_H: u32 = 188;
/// Where the boxes' feet are: the top of the shelf board.
pub const STORE_FOOT: f32 = 372.0;
/// Where boxes and loose carts stand: just above the board's front edge.
pub const STORE_STAND: f32 = STORE_FOOT - 4.0;
/// How big a box at the side is, against the one in the middle, and how
/// much it is darkened.
pub const STORE_SIDE_SCALE: f32 = 0.86;
pub const STORE_SIDE_SHADE: f32 = 0.42;
/// How much of each box beside the middle one shows at the screen's edge.
pub const STORE_SLIVER: f32 = 84.0;

/// How far below the top of the screen the aisle sign hangs.
const SIGN_Y: f32 = 4.0;
const SIGN_H: f32 = 26.0;
const SIGN_PAD: f32 = 22.0;
const SIGN_MIN_W: f32 = 130.0;
/// The widest the sign's lettering is set, so a long name still hangs
/// on the screen.
const SIGN_MAX_W: f32 = 520.0;
const TAB_MAX_W: f32 = 420.0;
const SIGN_PX: f32 = 17.0;
const SIGN_FACE_H: u32 = 22;
/// How long the sign takes to swing down into place when the aisle changes.
pub const SIGN_IN_MS: Millis = 320;
const SIGN_DROP: f32 = 22.0;
const CEILING_H: f32 = 8.0;
/// The gap between the sign and the game's name, and between the details
/// and the bay the boxes stand in.
const NAME_GAP: f32 = 4.0;
const BAY_GAP: f32 = 6.0;

const TAB_PX: f32 = 12.0;
const TAB_H: u32 = 16;

pub const TAG_H: u32 = 20;
const TAG_PX: f32 = 13.5;
const TAG_STRIPE: u32 = 5;
const TAG_PAD: u32 = 7;
/// How far under the shelf's rail the tags hang.
const TAG_DROP: f32 = 2.0;

/// The top of the screen, up in the band a 4:3 panel adds.
fn screen_top() -> f32 {
    -crate::shelf::band()
}

/// Where the store prints the game's name: just under the aisle sign.
pub fn store_name_top() -> f32 {
    screen_top() + SIGN_Y + SIGN_H + NAME_GAP
}

/// The highest the store's boxes reach: under the game's name and its
/// details, where the bay's header board is.
pub fn store_ceiling() -> f32 {
    store_name_top() + crate::details::DETAILS_DROP + crate::details::DETAILS_H as f32 + BAY_GAP
}

const YELLOW: [f32; 4] = [0.949, 0.761, 0.0, 1.0];
const SIGN_BLUE: [f32; 4] = [0.078, 0.282, 0.784, 1.0];
const SIGN_INK: [u8; 3] = [0xff, 0xd8, 0x1a];
const CHAIN: [f32; 4] = [0.70, 0.72, 0.80, 1.0];

const RAIL_Y: f32 = STORE_FOOT + 7.0;
const RAIL_H: f32 = 5.0;
const ACCENT_H: f32 = 5.0;
const CABINET_BOTTOM: f32 = 412.0;
const BAND_Y: f32 = 422.0;

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
    let bay_top = (store_ceiling() - 4.0).max(CEILING_H + 10.0).round();
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let c = pixel(x, y, bay_top);
            let at = ((y as u32 * w + x as u32) * 4) as usize;
            for k in 0..3 {
                rgba[at + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
    UndoFace { rgba, w, h }
}

fn pixel(x: i32, y: i32, bay_top: f32) -> [f32; 3] {
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
    if fy < bay_top - 8.0 {
        // The wall: dark blue, lighter high up and in the middle, in slats.
        let t = fy / STORE_FOOT;
        let base = mix(rgb([0x10, 0x24, 0x6c]), rgb([0x0a, 0x16, 0x4a]), t);
        let slat = if x % 60 == 0 { 1.07 } else { 1.0 };
        return scale(base, light * slat);
    }
    if fy < bay_top {
        // The header board over the bay: lighter blue with a yellow edge.
        if fy >= bay_top - 2.0 {
            return scale([YELLOW[0], YELLOW[1], YELLOW[2]], 0.9);
        }
        let t = (fy - (bay_top - 8.0)) / 6.0;
        let board = mix(rgb([0x24, 0x4a, 0xb4]), rgb([0x1a, 0x38, 0x92]), t);
        return scale(board, 0.6 + 0.4 * light);
    }
    if fy < STORE_FOOT - 3.0 {
        // The bay's back panel: darker, with a pegboard's dots, and the
        // header's shadow along its top.
        let shade = 0.55 + 0.45 * ((fy - bay_top) / 16.0).min(1.0);
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
    if (y as f32) >= near + 40.0 && (y as f32) < near + 43.0 {
        c = mix(c, rgb([0xd8, 0xb0, 0x10]), 0.85);
    }
    scale(c, lit)
}

/// How much a game has been played, by the colour down its tag's edge.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StatTier {
    /// Never played.
    New,
    Played,
    /// Played often: a favourite.
    Favourite,
}

pub use shelf_store::FAVOURITE_PLAYS;

/// What a game's shelf tag says, in place of a rental shop's price.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatTag {
    pub text: String,
    pub tier: StatTier,
}

impl StatTag {
    /// The tag for a game played `times` times, last at `last_wall` (unix
    /// seconds on the wall clock): NEW for one never played, the plays for
    /// the others as rentals and, under the middle box, the day of the last one too.
    pub fn new(times: u32, last_wall: i64, middle: bool) -> StatTag {
        let Some(sticker) = crate::rental::RentalSticker::new(times, last_wall) else {
            return StatTag {
                text: "NEW".into(),
                tier: StatTier::New,
            };
        };
        let (rented, last) = crate::rental::rental_lines(&sticker);
        let text = match middle {
            true => format!("{rented} \u{b7} {}", last.replacen("LAST PLAYED", "LAST", 1)),
            false => rented,
        };
        let tier = match times >= FAVOURITE_PLAYS {
            true => StatTier::Favourite,
            false => StatTier::Played,
        };
        StatTag { text, tier }
    }
}

fn stripe_of(tier: StatTier) -> [u8; 3] {
    match tier {
        StatTier::New => [0x2c, 0xb0, 0x4a],
        StatTier::Played => [0x28, 0x66, 0xe0],
        StatTier::Favourite => [0xd0, 0x30, 0x30],
    }
}

/// A shelf tag: a white label as wide as its words, with a stripe of its
/// tier's colour down its left edge.
pub fn stat_tag_face(tag: &StatTag) -> UndoFace {
    let h = TAG_H;
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: vec![255; (40 * h * 4) as usize],
            w: 40,
            h,
        };
    };
    let layout = text::fit(font, &tag.text, 400.0, 1, TAG_PX, TAG_PX);
    let set = layout
        .lines
        .first()
        .map_or(0.0, |l| text::line_width(font, l, layout.px, layout.tracking));
    let w = TAG_STRIPE + 2 * TAG_PAD + set.ceil() as u32;
    let stripe = stripe_of(tag.tier);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            // Clipped corners.
            let corner = (x < 2 || x >= w - 2) && (y < 2 || y >= h - 2);
            if corner {
                continue;
            }
            let c = if x < TAG_STRIPE {
                stripe
            } else {
                [0xf6, 0xf5, 0xee]
            };
            let at = ((y * w + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    let iw = w - TAG_STRIPE;
    let mut inner = vec![0u8; (iw * h * 4) as usize];
    text::draw_centred(&mut inner, iw, h, &layout, [0x0e, 0x18, 0x50]);
    for y in 0..h {
        for x in 0..iw {
            let s = ((y * iw + x) * 4) as usize;
            if inner[s + 3] == 0 {
                continue;
            }
            text::blend_pixel(
                &mut rgba,
                w,
                x + TAG_STRIPE,
                y,
                inner[s + 3] as u32,
                [inner[s], inner[s + 1], inner[s + 2]],
            );
        }
    }
    UndoFace { rgba, w, h }
}

/// The box picture at `path`, shrunk to fit the most the store draws a box.
pub fn store_box_face(path: &std::path::Path) -> Option<CartFace> {
    let (rgba, w, h) = crate::art::fit(path, STORE_BOX_W, STORE_BOX_H)?;
    Some(CartFace { rgba, w, h })
}

/// The NES's red, from its logo.
const NES_RED: [f32; 4] = [
    0xd2 as f32 / 255.0,
    0x22 as f32 / 255.0,
    0x2b as f32 / 255.0,
    1.0,
];

/// Logo colours for the consoles whose grey or black housings barely show
/// on the rail: the SNES's purple, the Game Boy's magenta and the Mega
/// Drive's crimson.
fn logo_colour(platform: Platform) -> Option<[f32; 4]> {
    let rgb = |r: u8, g: u8, b: u8| [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
    match platform {
        Platform::Nes => Some(NES_RED),
        Platform::Snes => Some(rgb(0x7b, 0x4f, 0xc4)),
        Platform::Gb | Platform::Gbc => Some(rgb(0xb8, 0x1f, 0x6e)),
        Platform::Md => Some(rgb(0xc8, 0x10, 0x2e)),
        _ => None,
    }
}

/// The colour a console's games are marked with along the shelf's rail.
pub fn accent_of(platform: Option<Platform>) -> [f32; 4] {
    if let Some(colour) = platform.and_then(logo_colour) {
        return colour;
    }
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

/// The console's name on the ticket at the foot of the store.
pub fn ticket_face(label: &str) -> UndoFace {
    runs_face(label, 16.0, 2.0, crate::store_footer::TICKET_INK, 22, 360.0)
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
    // On a 4:3 panel the screen reaches up past the picture's top edge:
    // the wall carries on up into that band and the ceiling sits at the
    // very top.
    let band = crate::shelf::band();
    match face {
        Some(tex) => {
            out.push(Draw::Tex {
                x: 0.0,
                y: 0.0,
                w: OUT_W as f32,
                h: OUT_H as f32,
                tex,
                alpha: 1.0,
            });
            if band > 0.0 {
                let row = |y: f32| y / OUT_H as f32;
                // A row of wall from just under the ceiling, stretched up
                // over the band and the picture's own ceiling.
                out.push(Draw::Part {
                    x: 0.0,
                    y: -band,
                    w: OUT_W as f32,
                    h: band + CEILING_H,
                    tex,
                    alpha: 1.0,
                    u0: 0.0,
                    v0: row(CEILING_H + 1.0),
                    u1: 1.0,
                    v1: row(CEILING_H + 2.0),
                });
                out.push(Draw::Part {
                    x: 0.0,
                    y: -band,
                    w: OUT_W as f32,
                    h: CEILING_H,
                    tex,
                    alpha: 1.0,
                    u0: 0.0,
                    v0: 0.0,
                    u1: 1.0,
                    v1: row(CEILING_H),
                });
            }
        }
        None => out.push(rect(
            0.0,
            -band,
            OUT_W as f32,
            OUT_H as f32 + band,
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

/// A shelf tag, `(texture, width, height)`, under the box standing at
/// `centre` (x), by how far off the middle it is (0 to 1).
pub fn draw_stat_tag(
    (tex, w, h): (TexId, u32, u32),
    centre: f32,
    off_middle: f32,
    out: &mut Vec<Draw>,
) {
    out.push(Draw::Tex {
        x: (centre - w as f32 / 2.0).round(),
        y: RAIL_Y + RAIL_H + ACCENT_H + TAG_DROP / 2.0,
        w: w as f32,
        h: h as f32,
        tex,
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
    let top = screen_top() + SIGN_Y - SIGN_DROP * (1.0 - rest);
    let w = (fw as f32 + 2.0 * SIGN_PAD).max(SIGN_MIN_W);
    let x = ((OUT_W as f32 - w) / 2.0).round();
    let a = 0.35 + 0.65 * rest;
    for cx in [x + 16.0, x + w - 18.0] {
        let ceiling = screen_top() + CEILING_H;
        out.push(rect(cx, ceiling, 2.0, top - ceiling, with_alpha(CHAIN, a)));
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

/// The console's slot, `rise` (0 to 1) of the way up from under the foot
/// of the screen to where it sits on the cart shelf.
pub fn draw_risen_slot(platform: Option<Platform>, rise: f32, out: &mut Vec<Draw>) {
    let rise = rise.clamp(0.0, 1.0);
    if rise <= 0.0 {
        return;
    }
    let mut slot = Vec::new();
    crate::slot_chrome::draw_empty_slot_for(platform, &mut slot);
    let top = slot
        .iter()
        .map(|d| match d {
            Draw::Rect { y, .. } | Draw::Tex { y, .. } | Draw::Turned { y, .. } | Draw::Part { y, .. } => *y,
        })
        .fold(OUT_H as f32, f32::min);
    let ease = 1.0 - (1.0 - rise) * (1.0 - rise);
    let dy = (OUT_H as f32 - top) * (1.0 - ease);
    for d in &mut slot {
        match d {
            Draw::Rect { y, .. } | Draw::Tex { y, .. } | Draw::Turned { y, .. } | Draw::Part { y, .. } => {
                *y += dy
            }
        }
    }
    out.extend(slot);
}
