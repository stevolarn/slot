//! Box view: the middle cart turns round, like a card, and comes back as
//! its game box, larger, over the dimmed shelf. The box turns again to
//! show its back, and from the back turns straight back into the cart. A
//! turn can be reversed half way.

use std::path::Path;

use crate::draw::{Draw, TexId, OUT_W};
use crate::hud::Millis;

/// How long a whole turn, cart to box, takes.
pub const BOX_TURN_MS: Millis = 460;
pub const BOX_MAX_W: u32 = 420;
pub const BOX_MAX_H: u32 = 300;
pub const BOX_DIM: [f32; 4] = [0.0, 0.0, 0.0, 0.55];
const SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
const SHADOW_PX: f32 = 8.0;

/// Where box view has got to, as a position along the turns: 0 is the
/// cart on the shelf, 1 the box front, 2 the box back. It moves towards
/// its target one turn per `BOX_TURN_MS`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BoxView {
    from: f32,
    target: f32,
    since: Millis,
    /// Turning from the back straight to the cart: the box shown on the
    /// way is its back.
    back_side: bool,
}

/// What box view shows for a box face: its picture and size.
pub type BoxFace = Option<(TexId, u32, u32)>;

impl BoxView {
    pub fn opened(now: Millis) -> Self {
        BoxView {
            from: 0.0,
            target: 1.0,
            since: now,
            back_side: false,
        }
    }

    pub fn position(&self, now: Millis) -> f32 {
        let d = now.saturating_sub(self.since) as f32 / BOX_TURN_MS as f32;
        if self.target >= self.from {
            (self.from + d).min(self.target)
        } else {
            (self.from - d).max(self.target)
        }
    }

    /// How far round from the cart to the box front: 0 to 1.
    pub fn turned(&self, now: Millis) -> f32 {
        self.position(now).min(1.0)
    }

    pub fn is_open(&self) -> bool {
        self.target > 0.0
    }

    /// Whether the back is, or is turning, to the front.
    pub fn on_back(&self, now: Millis) -> bool {
        self.target >= 2.0 || self.position(now) > 1.0 || self.back_side
    }

    fn to(self, target: f32, now: Millis) -> Self {
        let at = self.position(now);
        // From the back, all the way round: go straight back to the cart
        // with the back showing, not round by the front.
        if target == 0.0 && at >= 2.0 {
            return BoxView {
                from: 1.0,
                target,
                since: now,
                back_side: true,
            };
        }
        BoxView {
            from: at,
            target,
            since: now,
            back_side: self.back_side && at <= 1.0,
        }
    }

    /// B: cart to front, front to back, back to the cart.
    pub fn next(self, now: Millis) -> Self {
        let next = match self.target {
            t if t <= 0.0 => 1.0,
            t if t <= 1.0 => 2.0,
            _ => 0.0,
        };
        self.to(next, now)
    }

    /// Turn back to the cart from wherever it has got to.
    pub fn closed(self, now: Millis) -> Self {
        self.to(0.0, now)
    }

    /// Turn round to the box front again from wherever it has got to.
    pub fn reopened(self, now: Millis) -> Self {
        self.to(1.0, now)
    }

    /// Back to showing the cart, and finished turning.
    pub fn shut(&self, now: Millis) -> bool {
        self.target <= 0.0 && self.position(now) <= 0.0
    }

    /// How wide the cart is drawn while it turns away: 1 is flat on,
    /// 0 is edge on, after which the box is showing.
    pub fn cart_squash(&self, now: Millis) -> f32 {
        let t = self.turned(now);
        if t >= 0.5 {
            0.0
        } else {
            (t * std::f32::consts::PI).cos()
        }
    }

    /// The dimming and the box turning in over the cart at `cart`
    /// (x, y, width, height). `front` and `back` are the box's two sides;
    /// a side with no picture is drawn as a plain panel of `plain`.
    /// `details` is what is known of the game: with a back scan, a strip
    /// to lay along its foot; with none, the whole of a back to show in
    /// its place.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        now: Millis,
        cart: (f32, f32, f32, f32),
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
        let t = p.min(1.0);
        let ease = t * t * (3.0 - 2.0 * t);
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: crate::draw::OUT_H as f32,
            colour: [BOX_DIM[0], BOX_DIM[1], BOX_DIM[2], BOX_DIM[3] * ease],
        });
        if p < 0.5 {
            return;
        }
        // The box is the front's size; a back with no picture of its own
        // takes the front's shape.
        let Some((_, bw, bh)) = front else {
            return;
        };
        let (cx, cy, cw, ch) = cart;
        let (mid_x, mid_y) = (cx + cw / 2.0, cy + ch / 2.0);
        let (side, squash, grow) = if p <= 1.0 {
            let side = if self.back_side { back } else { front };
            (side, -(p * std::f32::consts::PI).cos(), (p - 0.5) * 2.0)
        } else {
            let q = p - 1.0;
            let squash = (q * std::f32::consts::PI).cos();
            match q < 0.5 {
                true => (front, squash, 1.0),
                false => (back, -squash, 1.0),
            }
        };
        let showing_back = (p > 1.5) || (p <= 1.0 && self.back_side);
        let start = ch / bh as f32;
        let scale = start + (1.0 - start) * grow;
        let (w, h) = (bw as f32 * scale, bh as f32 * scale);
        let x = mid_x + (OUT_W as f32 / 2.0 - mid_x) * grow - w * squash / 2.0;
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
