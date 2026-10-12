use shelf_gfx::{OUT_H, OUT_W};

use crate::hud::{HUD_INK, PLATE_H};
use crate::icon::{haloed, HALO_PX};
use crate::text;
use crate::CartFace;
use shelf_store::{DisplayMode, SortKey};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Toast {
    CardUnwritable,
    /// The shelves were put in a new order.
    Sorted(SortKey),
    /// The shelves are shown another way.
    Display(DisplayMode),
    /// The store's shelf holds loose carts (true) or boxes.
    StoreShelf(bool),
}

impl Toast {
    pub fn all() -> Vec<Toast> {
        let mut all = vec![Toast::CardUnwritable];
        all.extend(SortKey::ALL.map(Toast::Sorted));
        all.extend(DisplayMode::ALL.map(Toast::Display));
        all.extend([false, true].map(Toast::StoreShelf));
        all
    }

    pub fn index(self) -> usize {
        Self::all().iter().position(|t| *t == self).unwrap_or(0)
    }

    pub fn alarm(self) -> bool {
        self == Toast::CardUnwritable
    }

    pub fn detail(self) -> Option<&'static str> {
        match self {
            Toast::CardUnwritable => Some("Progress is not being saved"),
            _ => None,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Toast::CardUnwritable => "Can't write to SD card",
            Toast::Sorted(SortKey::Name) => "Sorted by name",
            Toast::Sorted(SortKey::Genre) => "Sorted by genre",
            Toast::Sorted(SortKey::Year) => "Sorted by year",
            Toast::Sorted(SortKey::Publisher) => "Sorted by publisher",
            Toast::Sorted(SortKey::Rating) => "Sorted by rating",
            Toast::Sorted(SortKey::Plays) => "Sorted by plays",
            Toast::Display(DisplayMode::Carts) => "Cart shelf",
            Toast::Display(DisplayMode::Store) => "Video store",
            Toast::Display(DisplayMode::Boxes) => "Box shelf",
            Toast::StoreShelf(false) => "Store: boxes",
            Toast::StoreShelf(true) => "Store: loose carts",
        }
    }
}

const TOAST_W: u32 = 240;
const TOAST_H: u32 = 22;
const TOAST_PX: f32 = 16.0;
const TOAST_MIN_PX: f32 = 12.0;

const ALARM_W: u32 = 560;
const ALARM_H: u32 = 44;
const ALARM_PX: f32 = 32.0;
const DETAIL_H: u32 = 30;
const DETAIL_PX: f32 = 20.0;

struct Setting {
    w: u32,
    h: u32,
    px: f32,
    min_px: f32,
    middle: f32,
}

fn setting(toast: Toast) -> Setting {
    match toast.alarm() {
        true => Setting {
            w: ALARM_W,
            h: ALARM_H,
            px: ALARM_PX,
            min_px: ALARM_PX,
            middle: OUT_H as f32 / 2.0,
        },
        false => Setting {
            w: TOAST_W,
            h: TOAST_H,
            px: TOAST_PX,
            min_px: TOAST_MIN_PX,
            middle: PLATE_H / 2.0,
        },
    }
}

pub fn toast_rect(toast: Toast) -> (f32, f32, f32, f32) {
    let (w, h) = toast_box(toast);
    let (w, h) = (w as f32, h as f32);
    let middle = setting(toast).middle;
    ((OUT_W as f32 - w) / 2.0, middle - h / 2.0, w, h)
}

pub fn toast_box(toast: Toast) -> (u32, u32) {
    let s = setting(toast);
    (s.w + 2 * HALO_PX, s.h + detail_h(toast) + 2 * HALO_PX)
}

fn detail_h(toast: Toast) -> u32 {
    match toast.detail() {
        Some(_) => DETAIL_H,
        None => 0,
    }
}

pub fn toast_face(toast: Toast) -> CartFace {
    let Some(font) = text::label_font() else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let s = setting(toast);
    let layout = text::fit(font, toast.text(), s.w as f32, 1, s.px, s.min_px);
    let mut cov = text::coverage(s.w, s.h, &layout);
    if let Some(detail) = toast.detail() {
        let layout = text::fit(font, detail, s.w as f32, 1, DETAIL_PX, DETAIL_PX);
        cov.extend(text::coverage(s.w, DETAIL_H, &layout));
    }
    haloed(&cov, s.w, s.h + detail_h(toast), HUD_INK)
}
