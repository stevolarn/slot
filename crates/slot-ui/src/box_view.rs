//! Box view: the middle cart turns round, like a card, and comes back as
//! its game box, larger, over the dimmed shelf. Turning back runs the same
//! way in reverse, and a turn can be reversed half way.

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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BoxView {
    from: f32,
    open: bool,
    since: Millis,
}

impl BoxView {
    pub fn opened(now: Millis) -> Self {
        BoxView {
            from: 0.0,
            open: true,
            since: now,
        }
    }

    /// How far round it is: 0 shows the cart, 1 the box.
    pub fn turned(&self, now: Millis) -> f32 {
        let d = now.saturating_sub(self.since) as f32 / BOX_TURN_MS as f32;
        match self.open {
            true => (self.from + d).min(1.0),
            false => (self.from - d).max(0.0),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Turn back towards the cart from wherever it has got to.
    pub fn closed(self, now: Millis) -> Self {
        BoxView {
            from: self.turned(now),
            open: false,
            since: now,
        }
    }

    /// Turn round to the box again from wherever it has got to.
    pub fn reopened(self, now: Millis) -> Self {
        BoxView {
            from: self.turned(now),
            open: true,
            since: now,
        }
    }

    /// Back to showing the cart, and finished turning.
    pub fn shut(&self, now: Millis) -> bool {
        !self.open && self.turned(now) <= 0.0
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
    /// (x, y, width, height). `face` is the box picture and its size.
    pub fn draw(
        &self,
        now: Millis,
        cart: (f32, f32, f32, f32),
        face: Option<(TexId, u32, u32)>,
        out: &mut Vec<Draw>,
    ) {
        let t = self.turned(now);
        if t <= 0.0 {
            return;
        }
        let ease = t * t * (3.0 - 2.0 * t);
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: crate::draw::OUT_H as f32,
            colour: [BOX_DIM[0], BOX_DIM[1], BOX_DIM[2], BOX_DIM[3] * ease],
        });
        if t < 0.5 {
            return;
        }
        let Some((tex, bw, bh)) = face else {
            return;
        };
        let squash = -(t * std::f32::consts::PI).cos();
        let (cx, cy, cw, ch) = cart;
        let (mid_x, mid_y) = (cx + cw / 2.0, cy + ch / 2.0);
        let grow = (t - 0.5) * 2.0;
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
        out.push(Draw::Tex {
            x,
            y,
            w: w * squash,
            h,
            tex,
            alpha: 1.0,
        });
    }
}

/// The box picture at `path`, fitted to the box view's size.
pub fn box_face(path: &Path) -> Option<crate::CartFace> {
    let (rgba, w, h) = crate::art::fit(path, BOX_MAX_W, BOX_MAX_H)?;
    Some(crate::CartFace { rgba, w, h })
}
