use std::sync::OnceLock;

use slot_gfx::{Draw, TexId, OUT_H, OUT_W};
use slot_store::Theme;
use slot_store::{Cart, Platform};

use crate::cart::{cart_box, label_colour, label_text, seated_box, SEATED_W};
use crate::footer::Printed;
use crate::icon::icon_box;
use crate::plate::HINT_H;
use crate::shelf::foot_y;

pub const ALERT_PX: f32 = 44.0;

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
    /// A band across the housing either side of the bay, and its trim:
    /// the NES's black stripe, the Mega Drive's gold line.
    stripe: Option<([f32; 4], Option<[f32; 4]>)>,
}

const fn hex(c: u32) -> [f32; 4] {
    [
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
        1.0,
    ]
}

fn look(platform: Option<Platform>) -> Look {
    match platform {
        Some(Platform::Nes) => Look {
            housing: hex(0xbfbebb),
            recess: hex(0x48484b),
            opening: hex(0x0a0a0b),
            edge: hex(0x8c8b88),
            flaps: None,
            stripe: Some((hex(0x161617), Some(hex(0xc4202c)))),
        },
        Some(Platform::Md) => Look {
            housing: hex(0x141416),
            recess: hex(0x26262a),
            opening: hex(0x030304),
            edge: hex(0x5c5c63),
            flaps: Some(hex(0x34343a)),
            stripe: Some((hex(0x1e1e21), Some(hex(0xc9a44c)))),
        },
        Some(Platform::Snes) => Look {
            housing: hex(0xcacace),
            recess: hex(0x9b9ba2),
            opening: hex(0x1b1b1f),
            edge: hex(0xe4e4e8),
            flaps: Some(hex(0x84848b)),
            stripe: None,
        },
        _ => Look {
            housing: housing(),
            recess: recess(),
            opening: opening(),
            edge: edge(),
            flaps: None,
            stripe: None,
        },
    }
}

const FLAP_H: f32 = 3.0;
const STRIPE_H: f32 = 12.0;
const TRIM_H: f32 = 2.0;

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
    pub alert: Option<(TexId, f32)>,
    pub dim: f32,
    pub screen: f32,
    pub game: bool,
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

        let chrome = 1.0 - self.screen.clamp(0.0, 1.0);
        let look = look(Some(self.cart.platform));

        draw_slot_back(look, chrome, out);

        let (cw, ch) = cart_box(self.cart.platform);
        let (cw, ch) = (cw as f32, ch as f32);

        let scale = self.scale.clamp(0.0, 1.0);
        let (w0, h0) = (cw * scale, ch * scale);
        let (sw, sh) = seated_box(self.cart.platform);
        let (sw, sh) = (sw as f32, sh as f32);
        let (foot0, seated_foot) = (foot_y(ch), SEATED_Y + sh);
        let catch = catch_at(foot0, seated_foot);
        let travel = travel(seat, catch);
        let k = (travel / catch).min(1.0);
        let (cw, ch) = (w0 + (sw - w0) * k, h0 + (sh - h0) * k);
        let centre = self.rest + w0 / 2.0;
        let x = centre + (CX - centre) * k - cw / 2.0;
        let y = foot0 + (seated_foot - foot0) * travel - ch;
        let cart_alpha = if seat >= 1.0 { chrome } else { 1.0 };
        out.push(match self.face {
            Some(tex) => Draw::Tex {
                x,
                y,
                w: cw,
                h: ch,
                tex,
                alpha: cart_alpha,
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
                        cart_alpha,
                    ],
                }
            }
        });

        if let Some((tex, alpha)) = self.alert {
            let (w, h) = icon_box(ALERT_PX);
            let (w, h) = (w as f32, h as f32);
            out.push(Draw::Tex {
                x: x + (cw - w) / 2.0,
                y: y + (ch - h) / 2.0,
                w,
                h,
                tex,
                alpha,
            });
        }

        if chrome < 1.0 {
            front_bands(look, |x, y, w, h, _| {
                out.push(Draw::Rect {
                    x,
                    y,
                    w,
                    h,
                    colour: [0.0, 0.0, 0.0, 1.0],
                });
            });
        }

        if self.game && self.screen > 0.0 {
            out.push(Draw::Game);
        }

        draw_slot_front(look, chrome, out);
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
    if let Some((stripe, trim)) = look.stripe {
        let y = BAND_Y + (MOUTH_H - STRIPE_H) / 2.0;
        band(0.0, y, BAY_X, STRIPE_H, stripe);
        band(right, y, w - right, STRIPE_H, stripe);
        if let Some(trim) = trim {
            let y = y + STRIPE_H - TRIM_H;
            band(0.0, y, BAY_X, TRIM_H, trim);
            band(right, y, w - right, TRIM_H, trim);
        }
    }

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
