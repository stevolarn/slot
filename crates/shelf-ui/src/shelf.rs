use shelf_gfx::{Draw, TexId, OUT_H, OUT_W};
use shelf_store::Cart;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::blow::Blow;
use crate::cart::{
    cart_box, cart_shape, contact_span, label_colour, label_text, CartShape, CART_H, CART_W,
};
use crate::dust::{draw_dust, seed_of, FRONT_SPECKS, SIDE_SPECKS};
use crate::hud::Millis;
use crate::plate::NAME_H;
use crate::shell::shell_for;
use crate::silhouette::{Console, GbShell};
use crate::slot_chrome::{draw_empty_slot_for, MOUTH_H};

const SIDE_SCALE: f32 = 0.78;
const SIDE_ALPHA: f32 = 0.55;

static SLACK: AtomicU32 = AtomicU32::new(0);

pub fn set_shelf_slack(px: f32) {
    SLACK.store(px.max(0.0).to_bits(), Ordering::Relaxed);
}

fn raise() -> f32 {
    f32::from_bits(SLACK.load(Ordering::Relaxed)) / 2.0
}

pub fn panel_middle_y() -> f32 {
    OUT_H as f32 / 2.0 + raise()
}

// Room above a resting cart for the game's name, which sits centred in
// the gap between the top of the screen and the cart.
const NAME_ROOM: f32 = NAME_H as f32 + 32.0;

pub fn rest_y(h: f32) -> f32 {
    let y = if h > CART_H as f32 {
        (OUT_H as f32 - MOUTH_H - h) / 2.0
    } else {
        (OUT_H + CART_H) as f32 / 2.0 - h
    };
    // On a 4:3 panel the shelf lifts into the extra height, but a tall
    // cart (Game Boy) must not rise into its own name: it drops to leave the
    // name room, as far as the slot's mouth allows.
    (y - raise()).max(NAME_ROOM.min(OUT_H as f32 - MOUTH_H - h))
}

pub fn foot_y(h: f32) -> f32 {
    rest_y(h) + h
}
const OMEGA: f32 = 16.0;
/// The longest slice the scroll spring is stepped by: well inside where a
/// spring this stiff stays steady.
const SPRING_STEP: f32 = 1.0 / 240.0;
/// The most time one update moves the scroll on; by then it has settled.
const MAX_STEP_GAP: f32 = 1.0;
/// The furthest, in carts, the scroll may trail the selection.
const MAX_LAG: f32 = 1.0;
const PART: f32 = 130.0;

const SLOTS: i32 = 3;

const REPEAT_DELAY_MS: Millis = 400;
const REPEAT_MS: [Millis; 4] = [110, 85, 65, 50];

pub struct Shelf {
    pub carts: Vec<Cart>,
    pub index: usize,
    pub scroll: f32,
    faces: Vec<Option<TexId>>,
    shadow: Option<TexId>,
    gb_shadow: Option<TexId>,
    gbc_shadow: Option<TexId>,
    nes_shadow: Option<TexId>,
    md_shadow: Option<TexId>,
    snes_shadow: Option<TexId>,
    shapes: Vec<CartShape>,
    /// How dusty each cart is, 0 to 1; empty when no dust is to be shown.
    dust: Vec<f32>,
    ride: f32,
    vel: f32,
    held: Option<(i32, Millis, usize)>,
}

impl Shelf {
    pub fn new(carts: Vec<Cart>) -> Self {
        Shelf {
            shapes: carts.iter().map(cart_shape).collect(),
            faces: vec![None; carts.len()],
            carts,
            index: 0,
            scroll: 0.0,
            shadow: None,
            gb_shadow: None,
            gbc_shadow: None,
            nes_shadow: None,
            md_shadow: None,
            snes_shadow: None,
            dust: Vec::new(),
            ride: 0.0,
            vel: 0.0,
            held: None,
        }
    }

    /// Set how dusty each cart is, in the carts' order. An empty list shows
    /// no dust.
    /// Whether a direction is held down, so the shelf will step again on
    /// its own.
    pub fn held(&self) -> bool {
        self.held.is_some()
    }

    pub fn shapes(&self) -> &[CartShape] {
        &self.shapes
    }

    pub fn set_dust(&mut self, dust: Vec<f32>) {
        self.dust = dust;
    }

    pub fn dust(&self) -> &[f32] {
        &self.dust
    }

    pub fn select(&mut self, i: usize) {
        self.index = i;
        self.scroll = i as f32;
        self.ride = i as f32;
        self.vel = 0.0;
    }

    pub fn set_shadow(&mut self, face: TexId) {
        self.shadow = Some(face);
    }

    pub fn set_gb_shadow(&mut self, shell: GbShell, face: TexId) {
        match shell {
            GbShell::Notched => self.gb_shadow = Some(face),
            GbShell::Rounded => self.gbc_shadow = Some(face),
        }
    }

    pub fn set_console_shadow(&mut self, console: Console, face: TexId) {
        match console {
            Console::Nes => self.nes_shadow = Some(face),
            Console::Md => self.md_shadow = Some(face),
            Console::Snes => self.snes_shadow = Some(face),
        }
    }

    pub fn set_faces(&mut self, faces: Vec<TexId>) {
        let mut faces = faces.into_iter();
        for face in &mut self.faces {
            *face = faces.next();
        }
    }

    pub fn face(&self, i: usize) -> Option<TexId> {
        self.faces.get(i).copied().flatten()
    }

    pub fn set_face(&mut self, i: usize, tex: TexId) -> Option<TexId> {
        self.faces.get_mut(i).and_then(|f| f.replace(tex))
    }

    pub fn take_face(&mut self, i: usize) -> Option<TexId> {
        self.faces.get_mut(i).and_then(Option::take)
    }

    pub fn on_screen(&self) -> Vec<usize> {
        self.around(SLOTS)
    }

    pub fn around(&self, radius: i32) -> Vec<usize> {
        let mut out = Vec::new();
        for step in 0..=radius {
            for off in [step, -step] {
                if let Some(i) = self.cart_at_offset(off) {
                    if !out.contains(&i) {
                        out.push(i);
                    }
                }
            }
        }
        out
    }

    pub fn find(&self, stem: &str) -> Option<(&Cart, Option<TexId>)> {
        let i = self.carts.iter().position(|c| c.stem == stem)?;
        Some((&self.carts[i], self.face(i)))
    }

    pub fn left(&mut self) {
        self.step(-1);
    }

    pub fn right(&mut self) {
        self.step(1);
    }

    pub fn jump_next_letter(&mut self) {
        self.jump(1);
    }

    pub fn jump_prev_letter(&mut self) {
        self.jump(-1);
    }

    fn start_of_letter(&self, from: usize) -> usize {
        let n = self.carts.len();
        let letter = shelf_store::initial(&self.carts[from].stem);
        let mut at = from;
        for _ in 0..n {
            let before = (at as i32 - 1).rem_euclid(n as i32) as usize;
            if shelf_store::initial(&self.carts[before].stem) != letter {
                break;
            }
            at = before;
        }
        at
    }

    fn jump(&mut self, dir: i32) {
        let n = self.carts.len();
        if n < 2 {
            return;
        }
        let wrap = |i: i32| i.rem_euclid(n as i32) as usize;
        let here = shelf_store::initial(&self.carts[self.index].stem);
        if self
            .carts
            .iter()
            .all(|c| shelf_store::initial(&c.stem) == here)
        {
            return;
        }
        let target = match dir > 0 {
            true => {
                let mut at = self.index;
                for _ in 0..n {
                    at = wrap(at as i32 + 1);
                    if shelf_store::initial(&self.carts[at].stem) != here {
                        break;
                    }
                }
                at
            }
            false => {
                let start = self.start_of_letter(self.index);
                match start == self.index {
                    true => self.start_of_letter(wrap(start as i32 - 1)),
                    false => start,
                }
            }
        };
        let ahead = (target as i32 - self.index as i32).rem_euclid(n as i32);
        let delta = match dir > 0 {
            true => ahead,
            false => ahead - n as i32,
        };
        self.index = target;
        self.ride += delta as f32;
    }

    pub fn hold_left(&mut self, now: Millis) {
        self.hold(-1, now);
    }

    pub fn hold_right(&mut self, now: Millis) {
        self.hold(1, now);
    }

    fn hold(&mut self, by: i32, now: Millis) {
        self.step(by);
        self.held = Some((by, now + REPEAT_DELAY_MS, 0));
    }

    pub fn release_left(&mut self) {
        self.release(-1);
    }

    pub fn release_right(&mut self) {
        self.release(1);
    }

    fn release(&mut self, by: i32) {
        if matches!(self.held, Some((held, _, _)) if held == by) {
            self.held = None;
        }
    }

    pub fn release_hold(&mut self) {
        self.held = None;
    }

    pub fn tick(&mut self, now: Millis) {
        let Some((by, due, fired)) = self.held else {
            return;
        };
        if now < due {
            return;
        }
        self.step(by);
        let rate = REPEAT_MS[fired.min(REPEAT_MS.len() - 1)];
        self.held = Some((by, now + rate, fired + 1));
    }

    fn step(&mut self, by: i32) {
        let n = self.carts.len();
        if n < 2 {
            return;
        }
        self.index = (self.index as i32 + by).rem_euclid(n as i32) as usize;
        self.ride += by as f32;
    }

    pub fn scroll_target(&self) -> f32 {
        let n = self.carts.len();
        if n == 0 {
            return 0.0;
        }
        let from = self.ride;
        let n = n as f32;
        from + (self.index as f32 - from + n / 2.0).rem_euclid(n) - n / 2.0
    }

    pub fn cart_at_offset(&self, off: i32) -> Option<usize> {
        let n = self.carts.len() as i32;
        if n == 0 {
            return None;
        }
        if n == 1 {
            return (off == 0).then_some(self.index);
        }
        Some((self.index as i32 + off).rem_euclid(n) as usize)
    }

    pub fn selected_at(&self) -> (f32, f32) {
        let w = self
            .carts
            .get(self.index)
            .map_or(CART_W, |c| cart_box(c.platform).0) as f32;
        let offset = self.scroll_target() - self.scroll;
        let scale = shrink(offset);
        (OUT_W as f32 / 2.0 + offset * w - w * scale / 2.0, scale)
    }

    /// Move the scroll on by `dt` seconds. The spring is stepped in small
    /// slices, so a long gap between frames, as after the loop has slept
    /// on a still shelf, settles it rather than flinging it off the screen.
    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, MAX_STEP_GAP);
        let steps = (dt / SPRING_STEP).ceil().max(1.0);
        let step = dt / steps;
        for _ in 0..steps as u32 {
            let accel =
                -2.0 * OMEGA * self.vel - OMEGA * OMEGA * (self.scroll - self.scroll_target());
            self.vel += accel * step;
            self.scroll += self.vel * step;
        }
        // A spin or a held direction can run the selection ahead faster than
        // the spring follows; never trail it so far that the carts around
        // it are off the screen.
        let lag = self.scroll_target() - self.scroll;
        if lag.abs() > MAX_LAG {
            self.scroll = self.scroll_target() - lag.signum() * MAX_LAG;
        }
    }

    pub fn draw(&self, shake: f32, out: &mut Vec<Draw>) {
        self.draw_row(None, shake, 0.0, 1.0, out);
        let platform = self.carts.get(self.index).map(|c| c.platform);
        draw_empty_slot_for(platform, out);
    }

    pub fn draw_row(
        &self,
        hidden: Option<&str>,
        shake: f32,
        recede: f32,
        dim: f32,
        out: &mut Vec<Draw>,
    ) {
        self.draw_row_posed(hidden, shake, recede, dim, (0.0, 0.0), 1.0, 1.0, 1.0, out);
    }

    /// The shelf with the middle cart being blown on: lifted and tipped
    /// to show its bottom, the breath going over it, and the empty slot
    /// under it.
    pub fn draw_blown(&self, shake: f32, blow: &Blow, now: Millis, out: &mut Vec<Draw>) {
        let pose = blow.pose(now);
        let dust = blow.dust_left(now);
        self.draw_row_posed(
            None,
            shake,
            0.0,
            1.0,
            pose.offset,
            1.0,
            pose.stand,
            dust,
            out,
        );
        let platform = self.carts.get(self.index).map(|c| c.platform);
        draw_empty_slot_for(platform, out);
        let (Some(i), Some((x, y, w, h))) = (self.cart_at_offset(0), self.selected_rect()) else {
            return;
        };
        let (dx, dy) = pose.offset;
        let bottom = (x + shake + dx, y + dy + h * pose.stand, w, pose.bottom_h(w));
        let shell = shell_for(&self.carts[i]).colour;
        blow.draw_bottom(now, bottom, shell, contact_span(self.shapes[i]), out);
        blow.draw_breath(now, bottom, out);
    }

    /// The shelf with the middle cart narrowed to `squash` of its width
    /// about its middle, as when it turns round to show its box, and the
    /// empty slot under it.
    pub fn draw_turned(&self, shake: f32, squash: f32, out: &mut Vec<Draw>) {
        self.draw_row_posed(None, shake, 0.0, 1.0, (0.0, 0.0), squash, 1.0, 1.0, out);
        let platform = self.carts.get(self.index).map(|c| c.platform);
        draw_empty_slot_for(platform, out);
    }

    /// Where the middle cart's top edge is: its left end, height and width.
    pub fn selected_top(&self) -> Option<(f32, f32, f32)> {
        self.selected_rect().map(|(x, y, w, _)| (x, y, w))
    }

    /// Where the middle cart is drawn: x, y, width and height.
    pub fn selected_rect(&self) -> Option<(f32, f32, f32, f32)> {
        let i = self.cart_at_offset(0)?;
        let offset = self.scroll_target() - self.scroll;
        let (cw, ch) = cart_box(self.carts[i].platform);
        let scale = shrink(offset);
        let (w, h) = (cw as f32 * scale, ch as f32 * scale);
        let x = OUT_W as f32 / 2.0 + offset * cw as f32 - w / 2.0;
        Some((x, foot_y(ch as f32) - h, w, h))
    }

    #[allow(clippy::too_many_arguments)]
    /// The row, with the middle cart pushed by `jiggle`, narrowed to
    /// `squash` of its width about its middle and showing `stand` of its
    /// height down from its top, with `dust_front` of its dust.
    fn draw_row_posed(
        &self,
        hidden: Option<&str>,
        shake: f32,
        recede: f32,
        dim: f32,
        jiggle: (f32, f32),
        squash: f32,
        stand: f32,
        dust_front: f32,
        out: &mut Vec<Draw>,
    ) {
        let recede = recede.clamp(0.0, 1.0);
        let dim = dim.clamp(0.0, 1.0);
        let target = self.scroll_target();
        for slot in -SLOTS..=SLOTS {
            let Some(i) = self.cart_at_offset(slot) else {
                continue;
            };
            let cart = &self.carts[i];
            if hidden == Some(cart.stem.as_str()) {
                continue;
            }
            let offset = target + slot as f32 - self.scroll;
            let t = offset.abs().min(1.0);
            let scale = shrink(offset);
            let alpha = (1.0 + (SIDE_ALPHA - 1.0) * t) * (1.0 - recede);
            let (cw, ch) = cart_box(cart.platform);
            let (w, h) = (cw as f32 * scale, ch as f32 * scale);
            let away = offset.signum() * (1.0 + offset.abs());
            let x = OUT_W as f32 / 2.0 + offset * cw as f32 - w / 2.0 + away * PART * recede;
            if x + w <= 0.0 || x >= OUT_W as f32 || alpha <= 0.0 {
                continue;
            }
            let (jx, jy) = if slot == 0 { jiggle } else { (0.0, 0.0) };
            let x = x + shake + jx;
            let (x, w) = match slot {
                0 if squash < 1.0 => {
                    if squash <= 0.0 {
                        continue;
                    }
                    (x + w * (1.0 - squash) / 2.0, w * squash)
                }
                _ => (x, w),
            };
            let y = foot_y(ch as f32) - h + jy;
            let h = if slot == 0 { h * stand } else { h };
            if alpha < 1.0 {
                let backing = match self.shapes.get(i).copied() {
                    None | Some(CartShape::Gba) => self.shadow,
                    Some(CartShape::Gb(GbShell::Notched)) => self.gb_shadow,
                    Some(CartShape::Gb(GbShell::Rounded)) => self.gbc_shadow,
                    Some(CartShape::Console(Console::Nes)) => self.nes_shadow,
                    Some(CartShape::Console(Console::Md)) => self.md_shadow,
                    Some(CartShape::Console(Console::Snes)) => self.snes_shadow,
                };
                if let Some(tex) = backing {
                    out.push(Draw::Tex {
                        x,
                        y,
                        w,
                        h,
                        tex,
                        alpha: recede_alpha(alpha),
                    });
                }
            }
            let face = match self.face(i) {
                Some(tex) => Draw::Tex {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha: alpha * dim,
                },
                None => {
                    let c = label_colour(&label_text(cart));
                    Draw::Rect {
                        x,
                        y,
                        w,
                        h,
                        colour: [
                            c[0] as f32 / 255.0,
                            c[1] as f32 / 255.0,
                            c[2] as f32 / 255.0,
                            alpha * dim,
                        ],
                    }
                }
            };
            out.push(face);
            let dust = self.dust.get(i).copied().unwrap_or(0.0);
            if dust > 0.0 {
                let (dust, specks) = match slot {
                    0 => (dust * dust_front, FRONT_SPECKS),
                    _ => (dust, SIDE_SPECKS),
                };
                draw_dust(
                    (x, y, w, h),
                    dust,
                    alpha * dim,
                    seed_of(&cart.stem),
                    specks,
                    out,
                );
            }
        }
    }
}

fn shrink(offset: f32) -> f32 {
    1.0 + (SIDE_SCALE - 1.0) * offset.abs().min(1.0)
}

fn recede_alpha(face_alpha: f32) -> f32 {
    (face_alpha / SIDE_ALPHA).clamp(0.0, 1.0)
}
