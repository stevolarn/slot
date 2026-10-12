use std::sync::OnceLock;

use shelf_gfx::{Draw, TexId, OUT_H, OUT_W};
use shelf_store::Theme;
use shelf_store::{Cart, Platform};

use crate::cart::{cart_box, label_colour, label_text, seated_box, stick_out, SEATED_W};
use crate::footer::Printed;
use crate::plate::HINT_H;
use crate::shelf::foot_y;

pub const MOUTH_H: f32 = 58.0;
const BAND_Y: f32 = OUT_H as f32 - MOUTH_H;

pub const MOUTH_W: f32 = SEATED_W as f32 + 14.0;
const SLIT_H: f32 = 9.0;
const MOUTH_X: f32 = (OUT_W as f32 - MOUTH_W) / 2.0;
const SLIT_Y: f32 = BAY_Y + 5.0;

pub const LIP_H: f32 = 2.0;

const BAY_W: f32 = MOUTH_W + 18.0;
const BAY_X: f32 = (OUT_W as f32 - BAY_W) / 2.0;
const BAY_Y: f32 = BAND_Y + LIP_H;
const SCOOP_W: f32 = MOUTH_W * 0.88;
const SCOOP_D: f32 = RECESS_H - (SCOOP_Y - BAY_Y);
const SCOOP_Y: f32 = SLIT_Y + SLIT_H;

const RECESS_H: f32 = 42.0;
const RIM_W: f32 = 2.0;
const CX: f32 = OUT_W as f32 / 2.0;
const SCOOP_FLAT: f32 = 4.0;

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn set_theme(theme: Theme) {
    let _ = THEME.set(theme);
}

pub fn theme() -> &'static Theme {
    THEME.get_or_init(Theme::default)
}

fn rgb(c: [u8; 3]) -> [f32; 4] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        1.0,
    ]
}

pub fn housing() -> [f32; 4] {
    rgb(theme().housing)
}

pub fn opening() -> [f32; 4] {
    rgb(theme().opening)
}

pub fn edge() -> [f32; 4] {
    rgb(theme().edge)
}

pub fn recess() -> [f32; 4] {
    rgb(theme().recess)
}

/// The colours a slot is drawn in. The handhelds take the card's theme;
/// the home consoles are drawn as their own cart slot.
#[derive(Clone, Copy)]
struct Look {
    housing: [f32; 4],
    recess: [f32; 4],
    opening: [f32; 4],
    edge: [f32; 4],
    /// The dust flaps either side of the slit, on the SNES and Mega Drive.
    flaps: Option<[f32; 4]>,
    /// Darker trim at both ends of the bar, where the battery and clock
    /// sit, so their light ink reads on a light console.
    ends: Option<[f32; 4]>,
    /// The console's buttons, lights, badges and lines, as (x, y, w, h,
    /// colour) with y from the top of the bar. They keep clear of the
    /// ends, where the battery and clock are.
    details: &'static [(f32, f32, f32, f32, u32)],
}

const fn hex(c: u32) -> [f32; 4] {
    [
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
        1.0,
    ]
}

/// How wide the darker ends are: the battery and clock sit inside them.
const END_W: f32 = 148.0;

const NES_DETAILS: &[(f32, f32, f32, f32, u32)] = &[
    (END_W, 44.0, OUT_W as f32 - 2.0 * END_W, 14.0, 0x858382),
    (END_W, 43.0, OUT_W as f32 - 2.0 * END_W, 1.0, 0x6e6c6b),
    (158.0, 49.0, 5.0, 5.0, 0xe0262d),
    (168.0, 47.0, 24.0, 8.0, 0x3a3a3b),
    (196.0, 47.0, 24.0, 8.0, 0x3a3a3b),
    (504.0, 20.0, 58.0, 14.0, 0xc4122f),
    (508.0, 25.0, 50.0, 4.0, 0xe8e6e2),
];

const SNES_DETAILS: &[(f32, f32, f32, f32, u32)] = &[
    (156.0, 18.0, 46.0, 20.0, 0x7e7d7e),
    (160.0, 21.0, 18.0, 14.0, 0x5f5e60),
    (163.0, 22.0, 2.0, 12.0, 0x8d8c8e),
    (168.0, 22.0, 2.0, 12.0, 0x8d8c8e),
    (173.0, 22.0, 2.0, 12.0, 0x8d8c8e),
    (504.0, 19.0, 8.0, 8.0, 0x2a5bd7),
    (514.0, 19.0, 8.0, 8.0, 0x2ea44f),
    (504.0, 29.0, 8.0, 8.0, 0xf3c22b),
    (514.0, 29.0, 8.0, 8.0, 0xd82b2b),
    (528.0, 18.0, 38.0, 20.0, 0x7e7d7e),
    (532.0, 21.0, 30.0, 14.0, 0x2a2a2c),
];

const MD_DETAILS: &[(f32, f32, f32, f32, u32)] = &[
    (END_W, 50.0, OUT_W as f32 - 2.0 * END_W, 2.0, 0xc9a44c),
    (156.0, 20.0, 5.0, 5.0, 0xb0232d),
    (166.0, 18.0, 34.0, 10.0, 0x2b2b2e),
    (170.0, 20.0, 12.0, 6.0, 0x4a4a4e),
    (166.0, 32.0, 50.0, 2.0, 0x26262a),
    (166.0, 37.0, 50.0, 2.0, 0x26262a),
    (500.0, 20.0, 62.0, 18.0, 0xd9b44a),
    (504.0, 26.0, 54.0, 6.0, 0x1a1a1a),
];

const GB_DETAILS: &[(f32, f32, f32, f32, u32)] = &[
    (154.0, 28.0, 66.0, 2.0, 0x8b1d5e),
    (154.0, 33.0, 66.0, 2.0, 0x2c2f6b),
    (500.0, 28.0, 66.0, 2.0, 0x8b1d5e),
    (500.0, 33.0, 66.0, 2.0, 0x2c2f6b),
    (156.0, 4.0, 40.0, 8.0, 0x9e9a95),
    (158.0, 5.0, 16.0, 6.0, 0xd8d4cf),
];

const GBC_DETAILS: &[(f32, f32, f32, f32, u32)] = &[
    (154.0, 34.0, 34.0, 16.0, 0x56704d),
    (158.0, 38.0, 10.0, 6.0, 0x2a2a30),
    (196.0, 40.0, 7.0, 7.0, 0xc4c4c8),
    (532.0, 34.0, 34.0, 16.0, 0x56704d),
    (552.0, 38.0, 10.0, 6.0, 0x2a2a30),
    (517.0, 40.0, 7.0, 7.0, 0xc4c4c8),
    (156.0, 4.0, 40.0, 8.0, 0x5f5287),
    (158.0, 5.0, 16.0, 6.0, 0xc7bce6),
];

const GBA_DETAILS: &[(f32, f32, f32, f32, u32)] = &[
    (0.0, 0.0, 104.0, 8.0, 0x9c97c4),
    (0.0, 7.0, 104.0, 1.0, 0x6e68a6),
    (616.0, 0.0, 104.0, 8.0, 0x9c97c4),
    (616.0, 7.0, 104.0, 1.0, 0x6e68a6),
    (156.0, 22.0, 5.0, 5.0, 0x3ee06a),
    (510.0, 30.0, 2.0, 18.0, 0x2e2668),
    (516.0, 30.0, 2.0, 18.0, 0x2e2668),
    (522.0, 30.0, 2.0, 18.0, 0x2e2668),
    (528.0, 30.0, 2.0, 18.0, 0x2e2668),
    (534.0, 30.0, 2.0, 18.0, 0x2e2668),
    (540.0, 30.0, 2.0, 18.0, 0x2e2668),
];

fn look(platform: Option<Platform>) -> Look {
    match platform {
        Some(Platform::Nes) => Look {
            housing: hex(0xc9c9c7),
            recess: hex(0x48484b),
            opening: hex(0x0a0a0b),
            edge: hex(0x8c8b88),
            flaps: None,
            ends: Some(hex(0x2e2e2e)),
            details: NES_DETAILS,
        },
        Some(Platform::Md) => Look {
            housing: hex(0x1c1c1e),
            recess: hex(0x141416),
            opening: hex(0x030304),
            edge: hex(0x5c5c63),
            flaps: Some(hex(0x34343a)),
            ends: None,
            details: MD_DETAILS,
        },
        Some(Platform::Snes) => Look {
            housing: hex(0xd1d1d1),
            recess: hex(0xa5a7a7),
            opening: hex(0x26262a),
            edge: hex(0xe4e4e8),
            flaps: Some(hex(0x84848b)),
            ends: Some(hex(0x66666c)),
            details: SNES_DETAILS,
        },
        Some(Platform::Gb) => Look {
            housing: hex(0xc5c1bc),
            recess: hex(0xa8a49f),
            opening: hex(0x2a2928),
            edge: hex(0xe0dcd7),
            flaps: None,
            ends: Some(hex(0x5d5f6c)),
            details: GB_DETAILS,
        },
        Some(Platform::Gbc) => Look {
            housing: hex(0x8a7bb3),
            recess: hex(0x6d5f99),
            opening: hex(0x241d36),
            edge: hex(0xb4a8d6),
            flaps: None,
            ends: Some(hex(0x4a3d78)),
            details: GBC_DETAILS,
        },
        Some(Platform::Gba) => Look {
            housing: hex(0x4b3f9b),
            recess: hex(0x3a307e),
            opening: hex(0x17142e),
            edge: hex(0x7a70cc),
            flaps: None,
            ends: Some(hex(0x352c78)),
            details: GBA_DETAILS,
        },
        None => Look {
            housing: housing(),
            recess: recess(),
            opening: opening(),
            edge: edge(),
            flaps: None,
            ends: None,
            details: &[],
        },
    }
}

/// The four colours a console's slot is drawn in: its housing, the bay
/// recess, the dark opening and the lip's edge.
pub fn slot_colours(platform: Option<Platform>) -> [[f32; 4]; 4] {
    let l = look(platform);
    [l.housing, l.recess, l.opening, l.edge]
}

const FLAP_H: f32 = 3.0;

const LIP_Y: f32 = BAND_Y;

const SEATED_Y: f32 = BAY_Y + 4.0;

fn catch_at(foot: f32, seated_foot: f32) -> f32 {
    (LIP_Y - foot) / (seated_foot - foot)
}

const CATCH_IN: f32 = 0.42;
const CATCH_OUT: f32 = 0.62;
const CREEP: f32 = 0.03;

pub struct SlotChrome<'a> {
    pub cart: &'a Cart,
    pub face: Option<TexId>,
    pub rest: f32,
    pub scale: f32,
    pub seat: f32,
    pub dim: f32,
}

impl SlotChrome<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        let seat = self.seat.clamp(0.0, 1.0);
        let dim = self.dim.clamp(0.0, 1.0);
        if dim > 0.0 {
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: OUT_W as f32,
                h: OUT_H as f32,
                colour: [0.0, 0.0, 0.0, dim],
            });
        }

        let look = look(Some(self.cart.platform));

        draw_slot_back(look, 1.0, out);

        let (cw, ch) = cart_box(self.cart.platform);
        let (cw, ch) = (cw as f32, ch as f32);

        let scale = self.scale.clamp(0.0, 1.0);
        let (w0, h0) = (cw * scale, ch * scale);
        let (sw, sh) = seated_box(self.cart.platform);
        let (sw, sh) = (sw as f32, sh as f32);
        let out_of_slot = stick_out(self.cart.platform) * sh;
        let (foot0, seated_foot) = (foot_y(ch), SEATED_Y + sh - out_of_slot);
        let catch = catch_at(foot0, seated_foot);
        let travel = travel(seat, catch);
        let k = (travel / catch).min(1.0);
        let (cw, ch) = (w0 + (sw - w0) * k, h0 + (sh - h0) * k);
        let centre = self.rest + w0 / 2.0;
        let x = centre + (CX - centre) * k - cw / 2.0;
        let y = foot0 + (seated_foot - foot0) * travel - ch;
        out.push(match self.face {
            Some(tex) => Draw::Tex {
                x,
                y,
                w: cw,
                h: ch,
                tex,
                alpha: 1.0,
            },
            None => {
                let c = label_colour(&label_text(self.cart));
                Draw::Rect {
                    x,
                    y,
                    w: cw,
                    h: ch,
                    colour: [
                        c[0] as f32 / 255.0,
                        c[1] as f32 / 255.0,
                        c[2] as f32 / 255.0,
                        1.0,
                    ],
                }
            }
        });

        draw_slot_front(look, 1.0, out);
    }
}

fn band(x: f32, y: f32, w: f32, h: f32, c: [f32; 4], alpha: f32) -> Draw {
    Draw::Rect {
        x,
        y,
        w,
        h,
        colour: [c[0], c[1], c[2], c[3] * alpha],
    }
}

fn draw_slot_back(look: Look, alpha: f32, out: &mut Vec<Draw>) {
    out.push(band(BAY_X, BAND_Y, BAY_W, LIP_H, look.housing, alpha));
    out.push(band(MOUTH_X, BAND_Y, MOUTH_W, LIP_H, look.edge, alpha));
    out.push(band(BAY_X, BAY_Y, BAY_W, RECESS_H, look.recess, alpha));
    if let Some(flap) = look.flaps {
        out.push(band(MOUTH_X, SLIT_Y - FLAP_H, MOUTH_W, FLAP_H, flap, alpha));
    }
    out.push(band(MOUTH_X, SLIT_Y, MOUTH_W, SLIT_H, look.opening, alpha));
    if let Some(flap) = look.flaps {
        out.push(band(MOUTH_X, SLIT_Y + SLIT_H, MOUTH_W, FLAP_H, flap, alpha));
    }
    for_each_scoop_span(|x, w, depth| {
        out.push(band(x, SCOOP_Y, w, depth, look.opening, alpha));
    });
}

fn draw_slot_front(look: Look, alpha: f32, out: &mut Vec<Draw>) {
    front_bands(look, |x, y, w, h, c| out.push(band(x, y, w, h, c, alpha)));
}

fn front_bands(look: Look, mut band: impl FnMut(f32, f32, f32, f32, [f32; 4])) {
    let w = OUT_W as f32;
    let right = BAY_X + BAY_W;
    let floor = SCOOP_Y + SCOOP_D + RIM_W;
    band(0.0, BAND_Y, BAY_X, MOUTH_H, look.housing);
    band(right, BAND_Y, w - right, MOUTH_H, look.housing);
    let near = CX - SCOOP_W / 2.0;
    band(BAY_X, SCOOP_Y, near - BAY_X, floor - SCOOP_Y, look.housing);
    let far = CX + SCOOP_W / 2.0;
    band(far, SCOOP_Y, right - far, floor - SCOOP_Y, look.housing);

    for_each_scoop_span(|x, w, depth| {
        let top = SCOOP_Y + depth + RIM_W;
        band(x, top, w, floor - top, look.housing);
    });
    band(0.0, floor, w, OUT_H as f32 - floor, look.housing);
    for_each_scoop_span(|x, w, depth| {
        band(x, SCOOP_Y + depth, w, RIM_W, look.edge);
    });
    if let Some(ends) = look.ends {
        band(0.0, BAND_Y, END_W, MOUTH_H, ends);
        band(w - END_W, BAND_Y, END_W, MOUTH_H, ends);
        band(END_W, BAND_Y, 1.5, MOUTH_H, look.edge);
        band(w - END_W - 1.5, BAND_Y, 1.5, MOUTH_H, look.edge);
    }
    for &(x, y, dw, dh, c) in look.details {
        band(x, BAND_Y + y, dw, dh, hex(c));
    }
}

fn for_each_scoop_span(mut span: impl FnMut(f32, f32, f32)) {
    let hw = SCOOP_W / 2.0;
    let depth = |x: f32| SCOOP_D * (1.0 - (x.abs() / hw).powf(SCOOP_FLAT)).max(0.0);
    let mut x = -hw;
    while x < hw {
        let d = depth(x + 0.5).round();
        let start = x;
        while x < hw && depth(x + 0.5).round() == d {
            x += 1.0;
        }
        span(CX + start, x - start, d);
    }
}

pub fn draw_slot_name(name: Printed, alpha: f32, out: &mut Vec<Draw>) {
    let (Some(tex), true) = (name.face, alpha > 0.0) else {
        return;
    };
    let (w, h) = (name.w as f32, HINT_H as f32);
    let hole = SCOOP_Y + SCOOP_D - SLIT_Y;
    out.push(Draw::Tex {
        x: CX - w / 2.0,
        y: SLIT_Y + (hole - h) / 2.0,
        w,
        h,
        tex,
        alpha,
    });
}

pub fn draw_empty_slot(out: &mut Vec<Draw>) {
    draw_empty_slot_for(None, out);
}

/// The empty slot of the console whose shelf is showing.
pub fn draw_empty_slot_for(platform: Option<Platform>, out: &mut Vec<Draw>) {
    let look = look(platform);
    draw_slot_back(look, 1.0, out);
    draw_slot_front(look, 1.0, out);
}

fn travel(seat: f32, catch: f32) -> f32 {
    if seat < CATCH_IN {
        catch * ease(seat / CATCH_IN)
    } else if seat < CATCH_OUT {
        catch + CREEP * (seat - CATCH_IN) / (CATCH_OUT - CATCH_IN)
    } else {
        let caught = catch + CREEP;
        caught + (1.0 - caught) * ease((seat - CATCH_OUT) / (1.0 - CATCH_OUT))
    }
}

pub fn ease(u: f32) -> f32 {
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}
