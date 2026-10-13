use shelf_gfx::{Draw, TexId, OUT_H, OUT_W};
use shelf_store::{Cart, Platform};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::blow::Blow;
use crate::boxes::{box_aspect, draw_wear, side_alpha, BoxesRow};
use crate::cart::{
    cart_box, cart_shape, contact_span, label_colour, label_text, CartShape, CART_H, CART_W,
};
use crate::dust::{draw_dust, seed_of, FRONT_SPECKS, SIDE_SPECKS};
use crate::hud::Millis;
use crate::plate::NAME_H;
use crate::shell::shell_for;
use crate::silhouette::{Console, GbShell};
use crate::slot_chrome::{draw_empty_slot_for, slot_colours, MOUTH_H};
use crate::store::{draw_stat_tag, STORE_SIDE_SHADE, STORE_STAND};

const SIDE_SCALE: f32 = 0.78;
const SIDE_ALPHA: f32 = 0.55;

static SLACK: AtomicU32 = AtomicU32::new(0);

pub fn set_shelf_slack(px: f32) {
    SLACK.store(px.max(0.0).to_bits(), Ordering::Relaxed);
}

fn raise() -> f32 {
    f32::from_bits(SLACK.load(Ordering::Relaxed)) / 2.0
}

/// How many canvas rows a 4:3 panel adds above the 720x480 picture, at
/// negative y: full-screen fills start there, and the game's name sits in
/// them.
pub fn band() -> f32 {
    f32::from_bits(SLACK.load(Ordering::Relaxed))
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
/// The most a loose cart in the store is drawn, against its size in the slot.
/// The room left between the store's boxes and the header above them.
const STORE_HEAD_GAP: f32 = 12.0;
const STORE_CART_GROW: f32 = 1.15;
/// How far the light round the store's middle box spreads, and in how many
/// steps it fades.
const STORE_GLOW_PX: f32 = 18.0;
const STORE_GLOW_STEPS: u32 = 6;

const REPEAT_DELAY_MS: Millis = 400;
const REPEAT_MS: [Millis; 4] = [110, 85, 65, 50];
/// After this many repeats (about two seconds held) a long shelf goes
/// `FAST_STRIDE` carts a step, so its far end is not a long wait away.
const FAST_AFTER: usize = 30;
const FAST_STRIDE: usize = 3;
const FAST_MIN_CARTS: usize = 60;

pub struct Shelf {
    pub carts: Vec<Cart>,
    pub index: usize,
    pub scroll: f32,
    faces: Vec<Option<TexId>>,
    /// The box fronts the store shelf shows: each with its size.
    box_faces: Vec<Option<(TexId, u32, u32)>>,
    /// The larger box fronts the box shelf shows: each with its size.
    tall_faces: Vec<Option<(TexId, u32, u32)>>,
    /// The console every game on the shelf is for, if they are all for one.
    platform: Option<Platform>,
    /// Which stretch of the shelf each cart is in, for jumping a stretch at
    /// a time; empty to go by the first letter of the name.
    groups: Vec<u32>,
    shadow: Option<TexId>,
    gb_shadow: Option<TexId>,
    gbc_shadow: Option<TexId>,
    nes_shadow: Option<TexId>,
    md_shadow: Option<TexId>,
    snes_shadow: Option<TexId>,
    gg_shadow: Option<TexId>,
    sms_shadow: Option<TexId>,
    n64_shadow: Option<TexId>,
    pce_shadow: Option<TexId>,
    pcecd_shadow: Option<TexId>,
    segacd_shadow: Option<TexId>,
    ps_shadow: Option<TexId>,
    ngp_shadow: Option<TexId>,
    ngpc_shadow: Option<TexId>,
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
            box_faces: vec![None; carts.len()],
            tall_faces: vec![None; carts.len()],
            platform: carts
                .first()
                .map(|c| c.platform)
                .filter(|p| carts.iter().all(|c| c.platform == *p)),
            groups: Vec::new(),
            carts,
            index: 0,
            scroll: 0.0,
            shadow: None,
            gb_shadow: None,
            gbc_shadow: None,
            nes_shadow: None,
            md_shadow: None,
            snes_shadow: None,
            gg_shadow: None,
            sms_shadow: None,
            n64_shadow: None,
            pce_shadow: None,
            pcecd_shadow: None,
            segacd_shadow: None,
            ps_shadow: None,
            ngp_shadow: None,
            ngpc_shadow: None,
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

    /// Whether the shelf has come to rest on its selection: no direction
    /// held and the scroll all but there.
    pub fn settled(&self) -> bool {
        self.held.is_none() && (self.scroll - self.scroll_target()).abs() < 0.5
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
            Console::Gg => self.gg_shadow = Some(face),
            Console::Sms => self.sms_shadow = Some(face),
            Console::N64 => self.n64_shadow = Some(face),
            Console::Pce => self.pce_shadow = Some(face),
            Console::PceCd => self.pcecd_shadow = Some(face),
            Console::SegaCd => self.segacd_shadow = Some(face),
            Console::Ps => self.ps_shadow = Some(face),
            Console::Ngp => self.ngp_shadow = Some(face),
            Console::Ngpc => self.ngpc_shadow = Some(face),
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

    /// The box front drawn for a cart on the store shelf, and its size.
    pub fn box_face(&self, i: usize) -> Option<(TexId, u32, u32)> {
        self.box_faces.get(i).copied().flatten()
    }

    pub fn set_box_face(&mut self, i: usize, tex: TexId, w: u32, h: u32) -> Option<TexId> {
        let old = self.box_faces.get_mut(i)?.replace((tex, w, h));
        old.map(|f| f.0)
    }

    pub fn take_box_face(&mut self, i: usize) -> Option<TexId> {
        self.box_faces
            .get_mut(i)
            .and_then(Option::take)
            .map(|f| f.0)
    }

    /// The box front drawn for a cart on the box shelf, and its size.
    pub fn tall_face(&self, i: usize) -> Option<(TexId, u32, u32)> {
        self.tall_faces.get(i).copied().flatten()
    }

    pub fn set_tall_face(&mut self, i: usize, tex: TexId, w: u32, h: u32) -> Option<TexId> {
        let old = self.tall_faces.get_mut(i)?.replace((tex, w, h));
        old.map(|f| f.0)
    }

    pub fn take_tall_face(&mut self, i: usize) -> Option<TexId> {
        self.tall_faces
            .get_mut(i)
            .and_then(Option::take)
            .map(|f| f.0)
    }

    /// Put the carts in a new order: `order[n]` is the place in the old
    /// order of the cart that is now `n`th. What each cart has (face, box,
    /// shape, dust) goes with it, and the selection stays on the cart it
    /// was on, with the scroll resting there.
    pub fn reorder(&mut self, order: &[usize]) {
        let n = self.carts.len();
        let mut seen = vec![false; n];
        if order.len() != n
            || order
                .iter()
                .any(|i| *i >= n || std::mem::replace(&mut seen[*i], true))
        {
            return;
        }
        let was = self.index;
        fn permute<T: Clone>(items: &[T], order: &[usize]) -> Vec<T> {
            order.iter().map(|i| items[*i].clone()).collect()
        }
        self.carts = permute(&self.carts, order);
        self.shapes = permute(&self.shapes, order);
        self.faces = permute(&self.faces, order);
        self.box_faces = permute(&self.box_faces, order);
        self.tall_faces = permute(&self.tall_faces, order);
        if !self.dust.is_empty() {
            self.dust = permute(&self.dust, order);
        }
        self.groups.clear();
        let at = order.iter().position(|i| *i == was).unwrap_or(0);
        self.select(at);
        self.held = None;
    }

    /// Say which stretch of the shelf each cart is in, in the carts'
    /// order; carts with the same number, side by side, are one stretch.
    pub fn set_groups(&mut self, groups: Vec<u32>) {
        self.groups = if groups.len() == self.carts.len() {
            groups
        } else {
            Vec::new()
        };
    }

    fn group(&self, i: usize) -> u32 {
        self.groups
            .get(i)
            .copied()
            .unwrap_or_else(|| shelf_store::initial(&self.carts[i].stem) as u32)
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
        let letter = self.group(from);
        let mut at = from;
        for _ in 0..n {
            let before = (at as i32 - 1).rem_euclid(n as i32) as usize;
            if self.group(before) != letter {
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
        let here = self.group(self.index);
        if (0..n).all(|i| self.group(i) == here) {
            return;
        }
        let target = match dir > 0 {
            true => {
                let mut at = self.index;
                for _ in 0..n {
                    at = wrap(at as i32 + 1);
                    if self.group(at) != here {
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
        let stride = match fired >= FAST_AFTER && self.carts.len() >= FAST_MIN_CARTS {
            true => FAST_STRIDE,
            false => 1,
        };
        for _ in 0..stride {
            self.step(by);
        }
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
        self.draw_row_posed(hidden, shake, recede, dim, (0.0, 0.0), 1.0, 1.0, 1.0, SLOTS, out);
    }

    /// The shelf with the middle cart being blown on: lifted and tipped
    /// to show its bottom, the breath going over it, and the empty slot
    /// under it.
    pub fn draw_blown(&self, shake: f32, blow: &Blow, now: Millis, out: &mut Vec<Draw>) {
        self.draw_blown_within(SLOTS, shake, blow, now, out);
    }

    /// As `draw_blown`, the middle cart alone: a cart out of its box.
    pub fn draw_middle_blown(&self, shake: f32, blow: &Blow, now: Millis, out: &mut Vec<Draw>) {
        self.draw_blown_within(0, shake, blow, now, out);
    }

    fn draw_blown_within(
        &self,
        reach: i32,
        shake: f32,
        blow: &Blow,
        now: Millis,
        out: &mut Vec<Draw>,
    ) {
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
            reach,
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
        blow.draw_bottom(now, bottom, shell, contact_span(self.shapes[i]), false, out);
        blow.draw_breath(now, bottom, out);
    }

    /// The shelf with the middle cart narrowed to `squash` of its width
    /// about its middle, as when it turns round to show its box, and the
    /// empty slot under it.
    pub fn draw_turned(&self, shake: f32, squash: f32, out: &mut Vec<Draw>) {
        self.draw_row_posed(None, shake, 0.0, 1.0, (0.0, 0.0), squash, 1.0, 1.0, SLOTS, out);
        let platform = self.carts.get(self.index).map(|c| c.platform);
        draw_empty_slot_for(platform, out);
    }

    /// The middle cart alone, where the cart shelf holds it, and the empty
    /// slot under it: a game taken out of its box on the box shelf.
    pub fn draw_middle(&self, shake: f32, out: &mut Vec<Draw>) {
        self.draw_row_posed(None, shake, 0.0, 1.0, (0.0, 0.0), 1.0, 1.0, 1.0, 0, out);
        let platform = self.carts.get(self.index).map(|c| c.platform);
        draw_empty_slot_for(platform, out);
    }

    /// How the store lays out this shelf under `ceiling`: its boxes, or with
    /// `carts` its loose carts, standing on the shelf board. A shelf of mixed
    /// consoles is laid out as the Mega Drive's boxes or the GBA's carts.
    pub fn store_row(&self, ceiling: f32, carts: bool) -> BoxesRow {
        // Clear of the header's line above and the shelf's edge below.
        let (ceiling, foot) = (ceiling + STORE_HEAD_GAP, STORE_STAND);
        if !carts {
            let aspect = box_aspect(self.platform.unwrap_or(Platform::Md));
            return BoxesRow::store(aspect, ceiling, foot);
        }
        let (cw, ch) = cart_box(self.platform.unwrap_or(Platform::Gba));
        let aspect = cw as f32 / ch as f32;
        let row = BoxesRow::store(aspect, ceiling, foot);
        // A loose cart is shown a little larger than in the slot, no more.
        let most = ch as f32 * STORE_CART_GROW;
        match row.h > most {
            true => BoxesRow::store(aspect, foot - most, foot),
            false => row,
        }
    }

    /// Where cart `i`'s box, or with `carts` the cart itself, stands in the
    /// store, `offset` places off the middle.
    fn store_rect(&self, row: &BoxesRow, carts: bool, i: usize, offset: f32) -> (f32, f32, f32, f32) {
        let size = match carts {
            true => {
                let (w, h) = cart_box(self.carts[i].platform);
                (w as f32, h as f32)
            }
            false => match self.tall_face(i) {
                Some((_, w, h)) => (w as f32, h as f32),
                None => (box_aspect(self.carts[i].platform), 1.0),
            },
        };
        row.rect(size, offset)
    }

    /// Where the middle box, or with `carts` the middle cart, is in the
    /// store.
    pub fn store_selected_rect(&self, ceiling: f32, carts: bool) -> Option<(f32, f32, f32, f32)> {
        let i = self.cart_at_offset(0)?;
        let row = self.store_row(ceiling, carts);
        Some(self.store_rect(&row, carts, i, self.scroll_target() - self.scroll))
    }

    /// The store shelf: the middle box (or with `carts`, cart) large and
    /// lit, an edge of the one either side, shaded, and a tag hung under
    /// each from `tag` (given the cart and whether it is the middle one).
    /// With `hide_middle` the middle one is left out, as while it is
    /// taken off the shelf; the others are shown at `others` (0 to 1).
    /// `blow`, with the time now, is a blow on the middle one: it shivers
    /// and a puff of breath goes over it, taking its dust.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_store_row(
        &self,
        ceiling: f32,
        carts: bool,
        tag: &dyn Fn(usize, bool) -> Option<(TexId, u32, u32)>,
        hide_middle: bool,
        others: f32,
        shake: f32,
        blow: Option<(Blow, Millis)>,
        out: &mut Vec<Draw>,
    ) {
        let row = self.store_row(ceiling, carts);
        let target = self.scroll_target();
        let mut slots: Vec<i32> = (-SLOTS..=SLOTS).collect();
        slots.sort_by_key(|s| std::cmp::Reverse(s.abs()));
        for slot in slots {
            let Some(i) = self.cart_at_offset(slot) else {
                continue;
            };
            if slot == 0 && hide_middle {
                continue;
            }
            let offset = target + slot as f32 - self.scroll;
            let (x, y, w, h) = self.store_rect(&row, carts, i, offset);
            let x = x + shake;
            if x + w <= 0.0 || x >= OUT_W as f32 {
                continue;
            }
            let off = offset.abs().min(1.0);
            let alpha = if slot == 0 { 1.0 } else { others.clamp(0.0, 1.0) };
            if alpha <= 0.0 {
                continue;
            }
            let cart = &self.carts[i];
            let blown = blow.filter(|_| slot == 0);
            // A cart blown on tips back to show its bottom, as on the cart
            // shelf; a box only shivers.
            let pose = blown.map(|(b, now)| b.pose(now));
            let stand = match (carts, pose) {
                (true, Some(p)) => p.stand,
                _ => 1.0,
            };
            let (x, y) = match blown {
                Some((b, now)) => {
                    let (dx, dy) = b.pose(now).offset;
                    (x + dx, y + dy)
                }
                None => (x, y),
            };
            if slot == 0 && !carts {
                // A soft pool of light behind the box being looked at.
                for k in 1..=STORE_GLOW_STEPS {
                    let spread = STORE_GLOW_PX * k as f32 / STORE_GLOW_STEPS as f32;
                    out.push(Draw::Rect {
                        x: x - spread,
                        y: y - spread * 0.7,
                        w: w + 2.0 * spread,
                        h: h + spread * 0.7,
                        colour: [0.55, 0.70, 1.0, 0.022 * alpha],
                    });
                }
            }
            if !carts {
                out.push(Draw::Rect {
                    x: x - 3.0,
                    y: y + h - 3.0,
                    w: w + 6.0,
                    h: 6.0,
                    colour: [0.0, 0.0, 0.05, 0.4 * alpha],
                });
            }
            let dust = self.dust.get(i).copied().unwrap_or(0.0);
            let dust = match blown {
                Some((b, now)) => dust * b.dust_left(now),
                None => dust,
            };
            if carts {
                let backing = self.backing(i);
                // A cart to the side is dimmed by letting its dark outline
                // show through it, as on the cart shelf, so nothing shows
                // round its shape. A clear Colour cart would show the dark
                // outline through its plastic and past its own rounder
                // corners, so it is only faded, the store showing through.
                // A photo of a NES, SNES or Mega Drive cart has its own
                // outline, not the drawn one, so it is faded too.
                let photo = matches!(self.shapes.get(i), Some(CartShape::Console(_)));
                let clear = (cart.platform == Platform::Gbc || photo) && self.face(i).is_some();
                let face_alpha = alpha * (1.0 - STORE_SIDE_SHADE * off);
                if off > 0.0 && !clear {
                    if let Some(tex) = backing {
                        out.push(Draw::Tex {
                            x,
                            y,
                            w,
                            h,
                            tex,
                            alpha,
                        });
                    }
                }
                let fh = h * stand;
                match self.face(i) {
                    Some(tex) => out.push(Draw::Tex {
                        x,
                        y,
                        w,
                        h: fh,
                        tex,
                        alpha: face_alpha,
                    }),
                    None => draw_placeholder(
                        (x, y, w, fh),
                        backing,
                        label_colour(&label_text(cart)),
                        face_alpha,
                        out,
                    ),
                }
                if dust > 0.0 {
                    let specks = if slot == 0 { FRONT_SPECKS } else { SIDE_SPECKS };
                    draw_dust((x, y, w, fh), dust, face_alpha, seed_of(&cart.stem), specks, out);
                }
                if let (Some((b, now)), Some(p), Some(shape)) = (blown, pose, self.shapes.get(i)) {
                    let bottom = (x, y + fh, w, p.bottom_h(w));
                    let shell = shell_for(cart).colour;
                    b.draw_bottom(now, bottom, shell, contact_span(*shape), true, out);
                    b.draw_breath(now, bottom, out);
                }
            } else {
                out.push(match self.tall_face(i) {
                    Some((tex, _, _)) => Draw::Tex {
                        x,
                        y,
                        w,
                        h,
                        tex,
                        alpha,
                    },
                    None => {
                        let c = slot_colours(Some(cart.platform))[0];
                        Draw::Rect {
                            x,
                            y,
                            w,
                            h,
                            colour: [c[0], c[1], c[2], alpha],
                        }
                    }
                });
                draw_wear((x, y, w, h), dust, alpha, seed_of(&cart.stem), out);
                if off > 0.0 {
                    out.push(Draw::Rect {
                        x,
                        y,
                        w,
                        h,
                        colour: [0.0, 0.0, 0.06, STORE_SIDE_SHADE * off * alpha],
                    });
                }
            }
            if let (Some((b, now)), false) = (blown, carts) {
                b.draw_breath(now, (x, y + h * 0.15, w, h * 0.3), out);
            }
            if let Some(face) = tag(i, slot == 0) {
                draw_stat_tag(face, x + w / 2.0, off, out);
            }
        }
    }

    /// How the box shelf lays out this shelf's boxes under `ceiling`: as
    /// its console's, or for a shelf of mixed consoles as the Mega Drive's,
    /// so the row holds still as it scrolls.
    pub fn boxes_row(&self, ceiling: f32) -> BoxesRow {
        BoxesRow::new(self.platform.unwrap_or(Platform::Md), ceiling)
    }

    /// Where box `i` stands on the box shelf, `offset` places off the
    /// middle. A box with no picture yet has its console's usual shape.
    fn boxes_rect(&self, row: &BoxesRow, i: usize, offset: f32) -> (f32, f32, f32, f32) {
        let size = match self.tall_face(i) {
            Some((_, w, h)) => (w as f32, h as f32),
            None => (box_aspect(self.carts[i].platform), 1.0),
        };
        row.rect(size, offset)
    }

    /// Where the middle box is on the box shelf.
    pub fn boxes_selected_rect(&self, ceiling: f32) -> Option<(f32, f32, f32, f32)> {
        let i = self.cart_at_offset(0)?;
        let row = self.boxes_row(ceiling);
        Some(self.boxes_rect(&row, i, self.scroll_target() - self.scroll))
    }

    /// The box shelf: the boxes in the dark, the middle one on top and the
    /// others dimmed, with shelf wear as dusty as each game is. With
    /// `hide_middle` the middle box is left out, as while it is opened;
    /// the others are shown at `others` (0 to 1).
    pub fn draw_boxes_row(
        &self,
        ceiling: f32,
        hide_middle: bool,
        others: f32,
        shake: f32,
        out: &mut Vec<Draw>,
    ) {
        let row = self.boxes_row(ceiling);
        let target = self.scroll_target();
        let mut slots: Vec<i32> = (-SLOTS..=SLOTS).collect();
        slots.sort_by_key(|s| std::cmp::Reverse(s.abs()));
        for slot in slots {
            let Some(i) = self.cart_at_offset(slot) else {
                continue;
            };
            if slot == 0 && hide_middle {
                continue;
            }
            let offset = target + slot as f32 - self.scroll;
            let (x, y, w, h) = self.boxes_rect(&row, i, offset);
            let x = x + shake;
            if x + w <= 0.0 || x >= OUT_W as f32 {
                continue;
            }
            let alpha = side_alpha(offset) * if slot == 0 { 1.0 } else { others.clamp(0.0, 1.0) };
            if alpha <= 0.0 {
                continue;
            }
            out.push(Draw::Rect {
                x: x + 6.0,
                y: y + 6.0,
                w,
                h,
                colour: [0.0, 0.0, 0.0, 0.45 * alpha],
            });
            out.push(match self.tall_face(i) {
                Some((tex, _, _)) => Draw::Tex {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha,
                },
                None => {
                    let c = slot_colours(Some(self.carts[i].platform))[0];
                    Draw::Rect {
                        x,
                        y,
                        w,
                        h,
                        colour: [c[0], c[1], c[2], alpha],
                    }
                }
            });
            let dust = self.dust.get(i).copied().unwrap_or(0.0);
            draw_wear((x, y, w, h), dust, alpha, seed_of(&self.carts[i].stem), out);
        }
    }

    /// The middle cart picked up off the store's shelf at `home` and
    /// carried `t` (0 to 1) of the way to `rest`, its place over the slot,
    /// lifted a little on the way.
    pub fn draw_store_lift(
        &self,
        home: (f32, f32, f32, f32),
        rest: (f32, f32, f32, f32),
        t: f32,
        out: &mut Vec<Draw>,
    ) {
        let Some(i) = self.cart_at_offset(0) else {
            return;
        };
        let t = t.clamp(0.0, 1.0);
        let e = t * t * (3.0 - 2.0 * t);
        let lerp = |a: f32, b: f32| a + (b - a) * e;
        let arc = 28.0 * (std::f32::consts::PI * t).sin();
        let (x, y, w, h) = (
            lerp(home.0, rest.0),
            lerp(home.1, rest.1) - arc,
            lerp(home.2, rest.2),
            lerp(home.3, rest.3),
        );
        out.push(Draw::Rect {
            x: x + 8.0 * e,
            y: y + 10.0 * e,
            w,
            h,
            colour: [0.0, 0.0, 0.0, 0.3 * (std::f32::consts::PI * t).sin()],
        });
        match self.face(i) {
            Some(tex) => out.push(Draw::Tex {
                x,
                y,
                w,
                h,
                tex,
                alpha: 1.0,
            }),
            None => draw_placeholder(
                (x, y, w, h),
                self.backing(i),
                label_colour(&label_text(&self.carts[i])),
                1.0,
                out,
            ),
        }
    }

    /// The outline of cart `i`'s shape, for its shadow and placeholder.
    fn backing(&self, i: usize) -> Option<TexId> {
        match self.shapes.get(i).copied() {
            None | Some(CartShape::Gba) => self.shadow,
            Some(CartShape::Gb(GbShell::Notched)) => self.gb_shadow,
            Some(CartShape::Gb(GbShell::Rounded)) => self.gbc_shadow,
            Some(CartShape::Console(Console::Nes)) => self.nes_shadow,
            Some(CartShape::Console(Console::Md)) => self.md_shadow,
            Some(CartShape::Console(Console::Snes)) => self.snes_shadow,
            Some(CartShape::Console(Console::Gg)) => self.gg_shadow,
            Some(CartShape::Console(Console::Sms)) => self.sms_shadow,
            Some(CartShape::Console(Console::N64)) => self.n64_shadow,
            Some(CartShape::Console(Console::Pce)) => self.pce_shadow,
            Some(CartShape::Console(Console::PceCd)) => self.pcecd_shadow,
            Some(CartShape::Console(Console::SegaCd)) => self.segacd_shadow,
            Some(CartShape::Console(Console::Ps)) => self.ps_shadow,
            Some(CartShape::Console(Console::Ngp)) => self.ngp_shadow,
            Some(CartShape::Console(Console::Ngpc)) => self.ngpc_shadow,
        }
    }

    /// The picture the middle cart is drawn with, if it has one.
    pub fn selected_face(&self) -> Option<TexId> {
        self.face(self.cart_at_offset(0)?)
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
    /// height down from its top, with `dust_front` of its dust; only the
    /// carts up to `reach` places off the middle.
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
        reach: i32,
        out: &mut Vec<Draw>,
    ) {
        let recede = recede.clamp(0.0, 1.0);
        let dim = dim.clamp(0.0, 1.0);
        let target = self.scroll_target();
        for slot in -reach..=reach {
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
            let backing = self.backing(i);
            if alpha < 1.0 {
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
            match self.face(i) {
                Some(tex) => out.push(Draw::Tex {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha: alpha * dim,
                }),
                None => draw_placeholder(
                    (x, y, w, h),
                    backing,
                    label_colour(&label_text(cart)),
                    alpha * dim,
                    out,
                ),
            }
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

/// Where a cart's label sits on a cart drawn before its face is made,
/// as fractions of the cart (x, y, width, height).
pub const PLACEHOLDER_LABEL: (f32, f32, f32, f32) = (0.12, 0.2, 0.76, 0.5);

/// A cart whose face is still being made: its outline in the dark, with
/// a panel of its label's colour where the label goes. Without the
/// outline to hand, the whole of it is the label's colour.
fn draw_placeholder(
    (x, y, w, h): (f32, f32, f32, f32),
    outline: Option<TexId>,
    label: [u8; 3],
    alpha: f32,
    out: &mut Vec<Draw>,
) {
    let colour = [
        label[0] as f32 / 255.0,
        label[1] as f32 / 255.0,
        label[2] as f32 / 255.0,
        alpha,
    ];
    let Some(tex) = outline else {
        out.push(Draw::Rect { x, y, w, h, colour });
        return;
    };
    out.push(Draw::Tex {
        x,
        y,
        w,
        h,
        tex,
        alpha: alpha * 0.85,
    });
    let (lx, ly, lw, lh) = PLACEHOLDER_LABEL;
    out.push(Draw::Rect {
        x: x + w * lx,
        y: y + h * ly,
        w: w * lw,
        h: h * lh,
        colour,
    });
}

fn shrink(offset: f32) -> f32 {
    1.0 + (SIDE_SCALE - 1.0) * offset.abs().min(1.0)
}

fn recede_alpha(face_alpha: f32) -> f32 {
    (face_alpha / SIDE_ALPHA).clamp(0.0, 1.0)
}
