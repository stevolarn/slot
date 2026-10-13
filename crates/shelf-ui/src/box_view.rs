//! Box view: the middle cart is put away in its game box, which comes up
//! larger over the dimmed shelf, front first. The box turns over to show
//! its back, and from the back turns over again and is opened: the flap
//! lifts, the cart is drawn up out of it and the box drops away as the
//! cart settles back onto the shelf. Every step can be reversed half way.

use std::f32::consts::PI;
use std::path::Path;

use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::hud::Millis;

/// How long the box takes to turn over, front to back or back to front.
pub const BOX_TURN_MS: Millis = 300;
/// How long the cart takes to be put in its box, or taken out of it.
pub const BOX_PACK_MS: Millis = 560;
pub const BOX_MAX_W: u32 = 420;
pub const BOX_MAX_H: u32 = 300;
pub const BOX_DIM: [f32; 4] = [0.0, 0.0, 0.0, 0.55];
const SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
const SHADOW_PX: f32 = 8.0;

// Taking the cart out runs from 0 to 1 through these stretches; putting
// it in is the same run backwards.
/// The flap lifting away.
const FLAP: (f32, f32) = (0.0, 0.24);
/// The cart rising out of the opening.
const RISE: (f32, f32) = (0.12, 0.55);
/// The box dropping away and the cart settling onto the shelf.
const LEAVE: (f32, f32) = (0.55, 1.0);
/// Of the leaving, how much the cart spends coming down; the rest is its
/// small hop on landing.
const LANDS: f32 = 0.75;
/// How much of the box's height its flap is.
const FLAP_FRAC: f32 = 0.2;
/// How dark the flap is when it is right open.
const FLAP_SHADE: f32 = 0.55;
/// The cart's size, against the shelf's, in the box and lifted out of it.
const IN_BOX_SCALE: f32 = 0.5;
const LIFTED_SCALE: f32 = 0.8;
/// The least gap above the cart when it is lifted out.
const LIFT_MARGIN: f32 = 8.0;
/// How far up out of the box, of its own height, the cart is lifted.
const LIFT_ABOVE: f32 = 0.6;
/// How the box shrinks and how far it drops as it goes.
const BOX_SHRINK: f32 = 0.25;
const BOX_DROP_PX: f32 = 70.0;
const HOP_PX: f32 = 6.0;
const OPENING: [f32; 4] = [0.04, 0.04, 0.05, 1.0];
const RIM: [f32; 4] = [0.32, 0.29, 0.26, 1.0];
const CART_SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 0.28];

/// Where box view has got to, as a position along its steps: 0 is the
/// cart on the shelf, 1 the box front, 2 the box back. From 0 to 1 the
/// cart is going into, or coming out of, its box; from 1 to 2 the box
/// turns over. It moves towards its target at the pace of the step it is
/// in.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BoxView {
    from: f32,
    target: f32,
    since: Millis,
}

/// What box view shows for a box face: its picture and size.
pub type BoxFace = Option<(TexId, u32, u32)>;

/// How far through a stretch `(from, to)` of 0 to 1 `t` is: 0 to 1.
fn span(t: f32, (from, to): (f32, f32)) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_in(t: f32) -> f32 {
    t * t
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl BoxView {
    pub fn opened(now: Millis) -> Self {
        BoxView {
            from: 0.0,
            target: 1.0,
            since: now,
        }
    }

    pub fn position(&self, now: Millis) -> f32 {
        let mut at = self.from;
        let mut left = now.saturating_sub(self.since) as f32;
        let up = self.target > at;
        while left > 0.0 && at != self.target {
            // The step `at` is in, and where it ends going this way.
            let packing = if up { at < 1.0 } else { at <= 1.0 };
            let end = match (up, packing) {
                (true, true) => self.target.min(1.0),
                (false, false) => self.target.max(1.0),
                _ => self.target,
            };
            let ms = if packing { BOX_PACK_MS } else { BOX_TURN_MS } as f32;
            let need = (end - at).abs() * ms;
            if left >= need {
                at = end;
                left -= need;
            } else {
                at += (end - at).signum() * left / ms;
                left = 0.0;
            }
        }
        at
    }

    pub fn is_open(&self) -> bool {
        self.target > 0.0
    }

    /// Whether the back is, or is turning, to the front.
    pub fn on_back(&self, now: Millis) -> bool {
        self.target >= 2.0 || self.position(now) > 1.0
    }

    fn to(self, target: f32, now: Millis) -> Self {
        BoxView {
            from: self.position(now),
            target,
            since: now,
        }
    }

    /// B: cart into its box, front to back, back to the cart.
    pub fn next(self, now: Millis) -> Self {
        let next = match self.target {
            t if t <= 0.0 => 1.0,
            t if t <= 1.0 => 2.0,
            _ => 0.0,
        };
        self.to(next, now)
    }

    /// Take the cart out of its box from wherever it has got to.
    pub fn closed(self, now: Millis) -> Self {
        self.to(0.0, now)
    }

    /// Back to showing the cart, and finished.
    pub fn shut(&self, now: Millis) -> bool {
        self.target <= 0.0 && self.position(now) <= 0.0
    }

    /// How wide the shelf draws its own middle cart: all of it when the
    /// cart is back on the shelf, none while box view draws the cart.
    pub fn cart_squash(&self, now: Millis) -> f32 {
        if self.position(now) <= 0.0 {
            1.0
        } else {
            0.0
        }
    }

    /// A peek on the box shelf: the box, standing in its place, is opened
    /// and its cart taken out.
    pub fn peeked(now: Millis) -> Self {
        BoxView {
            from: 1.0,
            target: 0.0,
            since: now,
        }
    }

    /// B on the box shelf with the box shut: turn it over to its back.
    pub fn flipped(now: Millis) -> Self {
        BoxView {
            from: 1.0,
            target: 2.0,
            since: now,
        }
    }

    /// B on the box shelf with a box in hand: the cart, if it is out, back
    /// into the box; the box's front turned to its back; its back turned
    /// to the front.
    pub fn flip(self, now: Millis) -> Self {
        let target = if self.target != 1.0 {
            1.0
        } else {
            2.0
        };
        self.to(target, now)
    }

    /// A on the box shelf: open the box, turning it to its front first if
    /// its back is showing, and take the cart out.
    pub fn open(self, now: Millis) -> Self {
        self.to(0.0, now)
    }

    /// Whether the box has got where it was going and is still.
    pub fn at_rest(&self, now: Millis) -> bool {
        self.position(now) == self.target
    }

    /// Put the cart back in its box and close it, from wherever the peek
    /// has got to.
    pub fn put_back(self, now: Millis) -> Self {
        self.to(1.0, now)
    }

    /// Whether a peek is over: the cart back in its box, the box shut and
    /// its front showing.
    pub fn put_away(&self, now: Millis) -> bool {
        self.target == 1.0 && self.position(now) == 1.0
    }

    /// Whether the cart is right out of its box, ready to play.
    pub fn taken_out(&self, now: Millis) -> bool {
        self.target <= 0.0 && self.position(now) <= 0.0
    }

    /// How far a peek is open: 0 the box shut, 1 the cart out.
    pub fn openness(&self, now: Millis) -> f32 {
        1.0 - self.position(now).clamp(0.0, 1.0)
    }

    /// A box in hand on the box shelf at `now`, where it stands at `home`
    /// (x, y, width, height): turning over to its `back` and again to its
    /// `front`, or opening, the cart rising out of it and coming down to
    /// `rest`, its place on the cart shelf, never higher than `ceiling`.
    /// Once the cart is out the shelf draws it, and this draws nothing.
    /// `cart_face`, `details` and `plain` are as for `draw`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_peek(
        &self,
        now: Millis,
        home: (f32, f32, f32, f32),
        rest: (f32, f32, f32, f32),
        cart_face: Option<TexId>,
        front: Option<TexId>,
        back: BoxFace,
        details: BoxFace,
        plain: [f32; 4],
        ceiling: f32,
        out: &mut Vec<Draw>,
    ) {
        let p = self.position(now);
        if p <= 0.0 {
            return;
        }
        let (x, y, w, h) = home;
        if p >= 1.0 {
            let side = front.map(|tex| (tex, w.round() as u32, h.round() as u32));
            let at = (x + w / 2.0, y + h / 2.0, w, h);
            return draw_turning(p, at, side, back, details, plain, out);
        }
        let Some(front) = front else {
            // No picture to open: the box is a panel, and the cart shows
            // over it once it is coming out.
            out.push(Draw::Rect { x, y, w, h, colour: plain });
            if p < 1.0 {
                let pose = Pose::at(1.0 - p, rest, Home::At(home), ceiling);
                draw_cart(pose.cart, cart_face, plain, out);
            }
            return;
        };
        Pose::at(1.0 - p, rest, Home::At(home), ceiling).draw(cart_face, front, plain, out);
    }

    /// The dimming, the cart and the box over the cart's place on the
    /// shelf at `cart` (x, y, width, height). `cart_face` is the picture
    /// the shelf draws it with; without one it is a panel of `plain`.
    /// `front` and `back` are the box's two sides; a side with no picture
    /// is drawn as a plain panel of `plain`. `details` is what is known of
    /// the game: with a back scan, a strip to lay along its foot; with
    /// none, the whole of a back to show in its place.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        now: Millis,
        cart: (f32, f32, f32, f32),
        cart_face: Option<TexId>,
        front: BoxFace,
        back: BoxFace,
        details: BoxFace,
        plain: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        let p = self.position(now);
        if p <= 0.0 {
            return;
        }
        let Some((front_tex, bw, bh)) = front else {
            // No box picture yet: the cart stays where it is.
            draw_cart(cart, cart_face, plain, out);
            return;
        };
        if p < 1.0 {
            let pose = Pose::at(1.0 - p, cart, Home::Centre(bw as f32, bh as f32), LIFT_MARGIN);
            return pose.draw(cart_face, front_tex, plain, out);
        }
        draw_sides(
            p,
            cart.1 + cart.3 / 2.0,
            (front_tex, bw, bh),
            front,
            back,
            details,
            plain,
            out,
        );
    }
}

/// How near the screen's edges a box turned over to its back comes.
const HELD_MARGIN: f32 = 10.0;

impl BoxView {
    /// The box, while it turns over or rests on its back, brought up to
    /// fill the screen over everything else: from where it stands at
    /// `home` (middle x, middle y, width, height) it grows and comes to
    /// the middle as it turns, and goes back as it turns to its front.
    /// `front` is its front's picture and size; the rest as for `draw`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_held_up(
        &self,
        now: Millis,
        home: (f32, f32, f32, f32),
        front: BoxFace,
        back: BoxFace,
        details: BoxFace,
        plain: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        let p = self.position(now);
        if p <= 1.0 {
            return;
        }
        let t = smooth((p - 1.0).clamp(0.0, 1.0));
        let band = crate::shelf::band();
        let (mx, my, w, h) = home;
        let room_h = OUT_H as f32 + band - 2.0 * HELD_MARGIN;
        let room_w = OUT_W as f32 - 2.0 * HELD_MARGIN;
        let grow = (room_w / w).min(room_h / h).max(1.0);
        let g = lerp(1.0, grow, t);
        let at = (
            lerp(mx, OUT_W as f32 / 2.0, t),
            lerp(my, -band + (OUT_H as f32 + band) / 2.0, t),
            w * g,
            h * g,
        );
        out.push(Draw::Rect {
            x: 0.0,
            y: -band,
            w: OUT_W as f32,
            h: OUT_H as f32 + band,
            colour: [0.0, 0.0, 0.0, t],
        });
        draw_turning(p, at, front, back, details, plain, out);
    }
}

/// The box standing at `p` (1 to 2) along box view's steps: dimming the
/// scene, then turning over from its front to its back, centred about
/// `mid_y`. `size` is the front's picture and size.
#[allow(clippy::too_many_arguments)]
fn draw_sides(
    p: f32,
    mid_y: f32,
    size: (TexId, u32, u32),
    front: BoxFace,
    back: BoxFace,
    details: BoxFace,
    plain: [f32; 4],
    out: &mut Vec<Draw>,
) {
    out.push(dim(1.0));
    let (_, bw, bh) = size;
    let at = (OUT_W as f32 / 2.0, mid_y, bw as f32, bh as f32);
    draw_turning(p, at, front, back, details, plain, out);
}

/// The box at `p` (1 to 2) along box view's steps, turning over from its
/// front to its back about its middle, at `at`: the middle's x and y, and
/// its width and height.
fn draw_turning(
    p: f32,
    (mid_x, mid_y, w, h): (f32, f32, f32, f32),
    front: BoxFace,
    back: BoxFace,
    details: BoxFace,
    plain: [f32; 4],
    out: &mut Vec<Draw>,
) {
    let q = (p - 1.0).clamp(0.0, 1.0);
    let squash = (q * PI).cos();
    let showing_back = q > 0.5;
    let (side, squash) = match showing_back {
        false => (front, squash),
        true => (back, -squash),
    };
    let x = mid_x - w * squash / 2.0;
    let y = mid_y - h / 2.0;
    out.push(Draw::Rect {
        x: x + SHADOW_PX * squash,
        y: y + SHADOW_PX,
        w: w * squash,
        h,
        colour: SHADOW,
    });
    match side {
        Some((tex, sw, sh)) if showing_back => {
            // A back scan fitted inside the front's outline.
            let fit = (w / sw as f32).min(h / sh as f32);
            let (dw, dh) = (sw as f32 * fit, sh as f32 * fit);
            out.push(Draw::Rect {
                x,
                y,
                w: w * squash,
                h,
                colour: plain,
            });
            out.push(Draw::Tex {
                x: x + (w - dw) * squash / 2.0,
                y: y + (h - dh) / 2.0,
                w: dw * squash,
                h: dh,
                tex,
                alpha: 1.0,
            });
            if let Some((strip, _, _)) = details {
                let strip_h = dh * crate::STRIP_FRAC;
                out.push(Draw::Tex {
                    x: x + (w - dw) * squash / 2.0,
                    y: y + (h - dh) / 2.0 + dh - strip_h,
                    w: dw * squash,
                    h: strip_h,
                    tex: strip,
                    alpha: 1.0,
                });
            }
        }
        Some((tex, _, _)) => out.push(Draw::Tex {
            x,
            y,
            w: w * squash,
            h,
            tex,
            alpha: 1.0,
        }),
        None => match details {
            Some((tex, _, _)) if showing_back => out.push(Draw::Tex {
                x,
                y,
                w: w * squash,
                h,
                tex,
                alpha: 1.0,
            }),
            _ => draw_plain_back(x, y, w * squash, h, plain, out),
        },
    }
}

fn dim(amount: f32) -> Draw {
    Draw::Rect {
        x: 0.0,
        y: -crate::shelf::band(),
        w: OUT_W as f32,
        h: OUT_H as f32 + crate::shelf::band(),
        colour: [BOX_DIM[0], BOX_DIM[1], BOX_DIM[2], BOX_DIM[3] * amount],
    }
}

fn draw_cart(
    rect: (f32, f32, f32, f32),
    face: Option<TexId>,
    plain: [f32; 4],
    out: &mut Vec<Draw>,
) {
    let (x, y, w, h) = rect;
    out.push(match face {
        Some(tex) => Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha: 1.0,
        },
        None => Draw::Rect {
            x,
            y,
            w,
            h,
            colour: plain,
        },
    });
}

/// Where the box is while its cart is taken out.
#[derive(Copy, Clone)]
enum Home {
    /// Held up over the cart's place, at this size, over the dimmed shelf.
    Centre(f32, f32),
    /// Standing where it is on the box shelf: x, y, width, height.
    At((f32, f32, f32, f32)),
}

/// Everything that moves while the cart is taken out of its box, at one
/// moment of it.
struct Pose {
    /// How far the flap is up: 0 shut, 1 right open.
    flap: f32,
    /// How much of the box is there, and how much of the dimming.
    box_alpha: f32,
    dim: f32,
    /// The box: x, y, width, height.
    box_rect: (f32, f32, f32, f32),
    cart: (f32, f32, f32, f32),
}

impl Pose {
    /// `out` is how far the cart is taken out of its box: 0 in, 1 on the
    /// shelf. `rest` is where the shelf holds it, `home` where the box is,
    /// and `ceiling` the highest the cart is lifted to.
    fn at(out: f32, rest: (f32, f32, f32, f32), home: Home, ceiling: f32) -> Pose {
        let (rx, ry, rw, rh) = rest;
        let flap = smooth(span(out, FLAP));
        let rise = ease_out(span(out, RISE));
        let leave = span(out, LEAVE);
        let gone = smooth(leave);
        let box_scale = 1.0 - BOX_SHRINK * gone;
        // The box's middle and size: centred over the cart's place on the
        // cart shelf, or where it stands on the box shelf.
        let ((mid_x, mid_y), (bw, bh), dim) = match home {
            Home::Centre(bw, bh) => ((OUT_W as f32 / 2.0, ry + rh / 2.0), (bw, bh), 1.0 - gone),
            Home::At((x, y, w, h)) => ((x + w / 2.0, y + h / 2.0), (w, h), 0.0),
        };
        let (w, h) = (bw * box_scale, bh * box_scale);
        let box_top = mid_y - bh / 2.0;
        let box_rect = (
            mid_x - w / 2.0,
            mid_y - h / 2.0 + BOX_DROP_PX * gone,
            w,
            h,
        );
        // The cart's top edge: just inside the box, then lifted out.
        let (in_w, in_h) = (rw * IN_BOX_SCALE, rh * IN_BOX_SCALE);
        let (up_w, up_h) = (rw * LIFTED_SCALE, rh * LIFTED_SCALE);
        let from_y = box_top + bh * FLAP_FRAC + 2.0;
        let to_y = (box_top - up_h * LIFT_ABOVE).max(ceiling);
        let (cw, ch) = (lerp(in_w, up_w, rise), lerp(in_h, up_h, rise));
        let (cx, cy) = (mid_x - cw / 2.0, lerp(from_y, to_y, rise));
        // Then down to the shelf, with a small hop on landing.
        let land = ease_in(span(leave, (0.0, LANDS)));
        let hop = HOP_PX * (span(leave, (LANDS, 1.0)) * PI).sin();
        let cart = (
            lerp(cx, rx, land),
            lerp(cy, ry, land) - hop,
            lerp(cw, rw, land),
            lerp(ch, rh, land),
        );
        Pose {
            flap,
            box_alpha: 1.0 - ease_out(span(leave, (0.0, 0.8))),
            dim,
            box_rect,
            cart,
        }
    }

    fn draw(&self, cart_face: Option<TexId>, front: TexId, plain: [f32; 4], out: &mut Vec<Draw>) {
        let a = self.box_alpha;
        let (x, y, w, h) = self.box_rect;
        let flap_h = h * FLAP_FRAC;
        out.push(dim(self.dim));
        out.push(Draw::Rect {
            x: x + SHADOW_PX,
            y: y + SHADOW_PX,
            w,
            h,
            colour: [SHADOW[0], SHADOW[1], SHADOW[2], SHADOW[3] * a],
        });
        // The dark inside of the box, under the flap, with the rim of the
        // box round it.
        if self.flap > 0.0 {
            let fade = |c: [f32; 4]| [c[0], c[1], c[2], c[3] * a];
            out.push(Draw::Rect {
                x,
                y,
                w,
                h: flap_h,
                colour: fade(RIM),
            });
            out.push(Draw::Rect {
                x: x + 4.0,
                y: y + 3.0,
                w: w - 8.0,
                h: flap_h - 3.0,
                colour: fade(OPENING),
            });
        }
        // The cart goes in before the box's front, so only what has come
        // up out of the opening shows.
        let (cx, cy, cw, ch) = self.cart;
        out.push(Draw::Rect {
            x: cx - 4.0,
            y: cy + 2.0,
            w: cw + 8.0,
            h: ch + 8.0,
            colour: CART_SHADOW,
        });
        draw_cart(self.cart, cart_face, plain, out);
        out.push(Draw::Part {
            x,
            y: y + flap_h,
            w,
            h: h - flap_h,
            tex: front,
            alpha: a,
            u0: 0.0,
            v0: FLAP_FRAC,
            u1: 1.0,
            v1: 1.0,
        });
        // The flap, hinged on the box's top edge, turning away.
        let up = flap_h * (1.0 - self.flap);
        if up >= 1.0 {
            out.push(Draw::Part {
                x,
                y,
                w,
                h: up,
                tex: front,
                alpha: a,
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: FLAP_FRAC,
            });
            out.push(Draw::Rect {
                x,
                y,
                w,
                h: up,
                colour: [0.0, 0.0, 0.0, FLAP_SHADE * self.flap * a],
            });
        }
    }
}

/// A box back with no scan: a panel in the console's colour with the
/// shapes a back has, a band of screenshots and the blurb under it.
fn draw_plain_back(x: f32, y: f32, w: f32, h: f32, colour: [f32; 4], out: &mut Vec<Draw>) {
    out.push(Draw::Rect { x, y, w, h, colour });
    if w < 8.0 {
        return;
    }
    let pale = [1.0, 1.0, 1.0, 0.18];
    for i in 0..3 {
        let sw = (w - 32.0) / 3.0;
        out.push(Draw::Rect {
            x: x + 8.0 + i as f32 * (sw + 8.0),
            y: y + h * 0.12,
            w: sw,
            h: h * 0.22,
            colour: [0.0, 0.0, 0.0, 0.35],
        });
    }
    for line in 0..6 {
        out.push(Draw::Rect {
            x: x + 12.0,
            y: y + h * 0.42 + line as f32 * h * 0.05,
            w: (w - 24.0) * if line == 5 { 0.6 } else { 1.0 },
            h: 3.0,
            colour: pale,
        });
    }
    out.push(Draw::Rect {
        x: x + w - 12.0 - w * 0.25,
        y: y + h * 0.82,
        w: w * 0.25,
        h: h * 0.1,
        colour: [1.0, 1.0, 1.0, 0.85],
    });
}

/// The box picture at `path`, fitted to the box view's size.
pub fn box_face(path: &Path) -> Option<crate::CartFace> {
    let (rgba, w, h) = crate::art::fit(path, BOX_MAX_W, BOX_MAX_H)?;
    Some(crate::CartFace { rgba, w, h })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CART: (f32, f32, f32, f32) = (180.0, 120.0, 360.0, 240.0);
    const CART_TEX: TexId = TexId::from_raw(1);
    const FRONT: TexId = TexId::from_raw(2);
    const BACK: TexId = TexId::from_raw(3);
    const BOX_H: f32 = 280.0;

    /// Taking the cart out from the box front, from `0`.
    const OUT_FROM_FRONT: BoxView = BoxView {
        from: 1.0,
        target: 0.0,
        since: 0,
    };

    fn draws(view: &BoxView, now: Millis) -> Vec<Draw> {
        let mut out = Vec::new();
        view.draw(
            now,
            CART,
            Some(CART_TEX),
            Some((FRONT, 400, 280)),
            Some((BACK, 400, 280)),
            None,
            [0.5; 4],
            &mut out,
        );
        out
    }

    fn cart_rect(out: &[Draw]) -> Option<(f32, f32, f32, f32)> {
        out.iter().find_map(|d| match *d {
            Draw::Tex {
                x, y, w, h, tex, ..
            } if tex == CART_TEX => Some((x, y, w, h)),
            _ => None,
        })
    }

    fn box_pieces(out: &[Draw]) -> Vec<Draw> {
        out.iter()
            .copied()
            .filter(
                |d| matches!(*d, Draw::Tex { tex, .. } | Draw::Part { tex, .. } if tex == FRONT),
            )
            .collect()
    }

    /// A view resting on the box back, then B at `at`.
    fn unboxing_from_back(at: Millis) -> BoxView {
        BoxView {
            from: 2.0,
            target: 2.0,
            since: 0,
        }
        .next(at)
    }

    #[test]
    fn the_steps_go_at_their_own_pace() {
        let view = BoxView::opened(0);
        assert_eq!(view.position(0), 0.0);
        assert!((view.position(BOX_PACK_MS / 2) - 0.5).abs() < 1e-4);
        assert_eq!(view.position(BOX_PACK_MS * 3), 1.0);
        let turning = view.next(BOX_PACK_MS);
        let half = BOX_PACK_MS + BOX_TURN_MS / 2;
        assert!((turning.position(half) - 1.5).abs() < 1e-4);
        assert_eq!(turning.position(half + BOX_TURN_MS), 2.0);
    }

    #[test]
    fn from_the_back_the_box_turns_over_then_opens_in_under_a_second() {
        let start = 10_000;
        let view = unboxing_from_back(start);
        assert_eq!(view.position(start), 2.0);
        assert!(view.on_back(start + 10));
        assert!(view.position(start + BOX_TURN_MS + 10) < 1.0);
        let total = BOX_TURN_MS + BOX_PACK_MS;
        assert!((700..=900).contains(&total), "takes {total} ms");
        assert!(view.shut(start + total));
        assert!(!view.shut(start + total - 20));
        assert_eq!(view.cart_squash(start + total), 1.0);
        assert_eq!(view.cart_squash(start + total - 20), 0.0);
    }

    #[test]
    fn at_the_start_of_taking_it_out_the_box_is_whole_and_the_cart_hidden() {
        let out = draws(&OUT_FROM_FRONT, 0);
        let whole = box_pieces(&out)
            .into_iter()
            .filter(|d| matches!(d, Draw::Tex { w, h, .. } if *w == 400.0 && *h == BOX_H))
            .count();
        assert_eq!(whole, 1, "the box front is not whole: {out:?}");
        assert!(cart_rect(&out).is_none(), "the cart shows already");
        // A moment on, the cart is in the box, behind the front.
        let soon = draws(&OUT_FROM_FRONT, 10);
        let box_top = CART.1 + CART.3 / 2.0 - BOX_H / 2.0;
        let (_, y, _, _) = cart_rect(&soon).expect("no cart");
        assert!(y > box_top + BOX_H * FLAP_FRAC, "cart outside its box");
    }

    #[test]
    fn half_way_the_flap_is_part_open_and_the_cart_part_out() {
        let out = draws(&OUT_FROM_FRONT, BOX_PACK_MS * 12 / 100);
        let flap = out
            .iter()
            .find_map(|d| match *d {
                Draw::Part { h, tex, v1, .. } if tex == FRONT && v1 <= FLAP_FRAC + 1e-4 => Some(h),
                _ => None,
            })
            .expect("no flap");
        assert!(flap > 0.0 && flap < BOX_H * FLAP_FRAC, "flap is {flap}");
        let out = draws(&OUT_FROM_FRONT, BOX_PACK_MS * 40 / 100);
        let (_, cy, _, ch) = cart_rect(&out).expect("no cart");
        let opening = CART.1 + CART.3 / 2.0 - BOX_H / 2.0 + BOX_H * FLAP_FRAC;
        assert!(cy < opening, "cart still hidden: {cy}");
        assert!(cy + ch > opening, "cart all the way out");
    }

    #[test]
    fn the_cart_ends_exactly_on_the_shelf_with_no_box_left() {
        let out = draws(&OUT_FROM_FRONT, BOX_PACK_MS - 1);
        let (x, y, w, h) = cart_rect(&out).expect("no cart");
        let (rx, ry, rw, rh) = CART;
        for (a, b) in [(x, rx), (y, ry), (w, rw), (h, rh)] {
            assert!((a - b).abs() < 1.0, "{a} against {b}");
        }
        let gone = box_pieces(&out).iter().all(|d| match *d {
            Draw::Part { alpha, .. } | Draw::Tex { alpha, .. } => alpha < 0.01,
            _ => true,
        });
        assert!(gone, "the box is still there");
        assert!(draws(&OUT_FROM_FRONT, BOX_PACK_MS).is_empty());
    }

    #[test]
    fn the_cart_lands_with_a_small_bump_that_comes_to_rest() {
        let ys: Vec<f32> = (0..BOX_PACK_MS)
            .filter_map(|t| cart_rect(&draws(&OUT_FROM_FRONT, t)).map(|r| r.1))
            .collect();
        // From its highest, it only comes down, to the shelf and no lower.
        let top = ys.iter().cloned().fold(f32::MAX, f32::min);
        let after = &ys[ys.iter().position(|y| *y == top).unwrap()..];
        let lowest = after.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            lowest <= CART.1 + 0.01,
            "it went through the shelf: {lowest}"
        );
        let rested = after.iter().position(|y| (*y - CART.1).abs() < 0.01);
        let rested = rested.expect("it never reached the shelf");
        assert!(
            after[rested..].iter().any(|y| *y < CART.1 - 1.0),
            "no bump on landing"
        );
    }

    #[test]
    fn putting_it_away_ends_on_the_box_front_at_full_size() {
        let view = BoxView::opened(0);
        assert!(draws(&view, 0).is_empty());
        let mid = draws(&view, BOX_PACK_MS / 2);
        assert!(cart_rect(&mid).is_some());
        let out = draws(&view, BOX_PACK_MS);
        let whole = box_pieces(&out).into_iter().any(|d| {
            matches!(d, Draw::Tex { w, h, alpha, .. }
                if (w - 400.0).abs() < 0.01 && (h - BOX_H).abs() < 0.01 && alpha == 1.0)
        });
        assert!(whole, "{out:?}");
        assert!(cart_rect(&out).is_none(), "the cart is outside its box");
    }

    #[test]
    fn turning_back_half_way_through_carries_on_from_where_it_got_to() {
        let view = unboxing_from_back(0);
        let at = BOX_TURN_MS + BOX_PACK_MS / 2;
        let mid = view.position(at);
        assert!(mid > 0.0 && mid < 1.0);
        let again = view.next(at);
        assert_eq!(again.position(at), mid);
        assert!(again.position(at + 50) > mid, "B did not put it back");
        assert_eq!(again.position(at + BOX_PACK_MS * 2), 1.0);
        let shut = view.closed(at + 100);
        assert!(shut.shut(at + 100 + BOX_PACK_MS));
    }

    #[test]
    fn closing_from_the_back_turns_over_first_and_wants_the_back_till_then() {
        let view = BoxView {
            from: 2.0,
            target: 2.0,
            since: 0,
        };
        let closing = view.closed(0);
        assert!(closing.on_back(10));
        assert!(!closing.on_back(BOX_TURN_MS + 10));
    }

    #[test]
    fn a_peek_opens_from_the_box_in_its_place_and_lands_the_cart_at_rest() {
        let home = (276.0, 90.0, 168.0, 240.0);
        let rest = (200.0, 61.0, 320.0, 240.0);
        let start = Pose::at(0.0, rest, Home::At(home), 30.0);
        assert_eq!(start.box_rect, home);
        assert_eq!(start.dim, 0.0, "a peek dims the whole screen");
        let end = Pose::at(1.0, rest, Home::At(home), 30.0);
        let close = |a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)| {
            (a.0 - b.0).abs() < 0.01
                && (a.1 - b.1).abs() < 0.01
                && (a.2 - b.2).abs() < 0.01
                && (a.3 - b.3).abs() < 0.01
        };
        assert!(close(end.cart, rest), "{:?}", end.cart);
        for k in 0..=100 {
            let pose = Pose::at(k as f32 / 100.0, rest, Home::At(home), 30.0);
            assert!(pose.cart.1 >= 30.0 - HOP_PX, "over the ceiling at {k}: {:?}", pose.cart);
        }
    }

    #[test]
    fn a_peek_runs_from_shut_to_out_and_back_to_put_away() {
        let peek = BoxView::peeked(0);
        assert!(!peek.taken_out(0) && peek.openness(0) == 0.0);
        assert!(peek.taken_out(BOX_PACK_MS));
        let back = peek.put_back(BOX_PACK_MS);
        assert!(!back.put_away(BOX_PACK_MS + 10));
        assert!(back.put_away(2 * BOX_PACK_MS));
        assert_eq!(back.openness(2 * BOX_PACK_MS), 0.0);
    }
}
