use slot_gfx::{Draw, TexId, OUT_H, OUT_W};
use slot_store::{BRIGHTNESS_MAX, VOLUME_MAX};

use crate::icon::icon_box;
use crate::toast::toast_rect;
use crate::{Icon, Toast};

pub type Millis = u64;

pub const HUD_MS: Millis = 1500;
const FADE_MS: Millis = 250;
const PULSE_MS: Millis = 500;

pub const PLATE_H: f32 = 40.0;
pub(crate) const PLATE: [f32; 4] = [0.0, 0.0, 0.0, 0.72];
pub const ALARM_BAND_H: f32 = 120.0;
const ALARM_BRIGHT: [f32; 4] = [0.82, 0.10, 0.10, 0.94];
const ALARM_DIM: [f32; 4] = [0.40, 0.03, 0.03, 0.94];

pub const HUD_ICON_PX: f32 = 24.0;
pub const HUD_INK: [u8; 3] = [0xf5, 0xf2, 0xef];
const ICON_GAP: f32 = 10.0;

const BAR_W: f32 = 320.0;
const BAR_H: f32 = 6.0;
const BAR_Y: f32 = (PLATE_H - BAR_H) / 2.0;
pub(crate) const TRACK: [f32; 4] = [1.0, 1.0, 1.0, 0.18];
pub(crate) const FILL: [f32; 4] = [
    HUD_INK[0] as f32 / 255.0,
    HUD_INK[1] as f32 / 255.0,
    HUD_INK[2] as f32 / 255.0,
    1.0,
];

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum HudKind {
    #[default]
    Brightness,
    Volume,
}

impl HudKind {
    pub fn icon(self, value: u8, muted: bool) -> Icon {
        match self {
            HudKind::Brightness => Icon::Brightness,
            HudKind::Volume if muted => Icon::VolumeMuted,
            HudKind::Volume if value == 0 => Icon::VolumeZero,
            HudKind::Volume => Icon::Volume,
        }
    }

    fn max(self) -> u8 {
        match self {
            HudKind::Brightness => BRIGHTNESS_MAX,
            HudKind::Volume => VOLUME_MAX,
        }
    }
}

#[derive(Default)]
pub struct Hud {
    pub kind: HudKind,
    pub value: u8,
    pub shown_at: Option<Millis>,
    muted: bool,
    said: Option<(Toast, Millis)>,
    alarm: Option<Millis>,
    icons: Vec<TexId>,
    toasts: Vec<TexId>,
}

impl Hud {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_icons(&mut self, icons: Vec<TexId>) {
        self.icons = icons;
    }

    pub fn set_toasts(&mut self, toasts: Vec<TexId>) {
        self.toasts = toasts;
    }

    pub fn toast(&mut self, toast: Toast, now: Millis) {
        self.said = Some((toast, now));
    }

    pub fn set_alarm(&mut self, on: bool, now: Millis) {
        self.alarm = match (on, self.alarm) {
            (true, Some(since)) => Some(since),
            (true, None) => Some(now),
            (false, _) => None,
        };
    }

    pub fn alarmed(&self) -> bool {
        self.alarm.is_some()
    }

    pub fn toast_visible(&self, now: Millis) -> bool {
        self.toast_alpha(now) > 0.0
    }

    pub fn said(&self, now: Millis) -> Option<Toast> {
        self.toast_visible(now)
            .then(|| self.said.map(|(t, _)| t))
            .flatten()
    }

    pub fn show(&mut self, kind: HudKind, value: u8, muted: bool, now: Millis) {
        self.kind = kind;
        self.value = value;
        self.muted = muted;
        self.shown_at = Some(now);
    }

    pub fn glyph(&self) -> Icon {
        self.kind.icon(self.value, self.muted)
    }

    pub fn visible(&self, now: Millis) -> bool {
        self.alpha(now) > 0.0
    }

    pub fn draw(&self, now: Millis, out: &mut Vec<Draw>) {
        let alpha = self.alpha(now);
        let toast = self.toast_alpha(now);
        if alpha > 0.0 || toast > 0.0 {
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: OUT_W as f32,
                h: PLATE_H,
                colour: faded(PLATE, alpha.max(toast)),
            });
            if toast > 0.0 {
                self.draw_toast(now, out);
            } else {
                self.draw_bar(alpha, out);
            }
        }
        if let Some(since) = self.alarm {
            self.draw_alarm(now.saturating_sub(since), out);
        }
    }

    fn draw_bar(&self, alpha: f32, out: &mut Vec<Draw>) {
        let x = (OUT_W as f32 - BAR_W) / 2.0;
        if let Some(tex) = self.icon() {
            let (w, h) = icon_box(HUD_ICON_PX);
            out.push(Draw::Tex {
                x: x - ICON_GAP - w as f32,
                y: (PLATE_H - h as f32) / 2.0,
                w: w as f32,
                h: h as f32,
                tex,
                alpha,
            });
        }
        out.push(Draw::Rect {
            x,
            y: BAR_Y,
            w: BAR_W,
            h: BAR_H,
            colour: faded(TRACK, alpha),
        });
        out.push(Draw::Rect {
            x,
            y: BAR_Y,
            w: BAR_W * self.fraction(),
            h: BAR_H,
            colour: faded(FILL, alpha),
        });
    }

    fn draw_alarm(&self, age: Millis, out: &mut Vec<Draw>) {
        out.push(Draw::Rect {
            x: 0.0,
            y: (OUT_H as f32 - ALARM_BAND_H) / 2.0,
            w: OUT_W as f32,
            h: ALARM_BAND_H,
            colour: pulse(age),
        });
        let alarm = Toast::CardUnwritable;
        let Some(tex) = self.toasts.get(alarm.index()).copied() else {
            return;
        };
        let (x, y, w, h) = toast_rect(alarm);
        out.push(Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha: 1.0,
        });
    }

    fn draw_toast(&self, now: Millis, out: &mut Vec<Draw>) {
        let alpha = self.toast_alpha(now);
        if alpha <= 0.0 {
            return;
        }
        let Some(tex) = self
            .said
            .and_then(|(t, _)| self.toasts.get(t.index()))
            .copied()
        else {
            return;
        };
        let Some((said, _)) = self.said else {
            return;
        };
        let (x, y, w, h) = toast_rect(said);
        out.push(Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha,
        });
    }

    fn icon(&self) -> Option<TexId> {
        self.icons.get(self.glyph().index()).copied()
    }

    fn fraction(&self) -> f32 {
        let max = self.kind.max();
        if max == 0 {
            return 0.0;
        }
        self.value.min(max) as f32 / max as f32
    }

    fn alpha(&self, now: Millis) -> f32 {
        let Some(shown) = self.shown_at else {
            return 0.0;
        };
        fade(shown, now)
    }

    fn toast_alpha(&self, now: Millis) -> f32 {
        match self.said {
            Some((_, shown)) => fade(shown, now),
            None => 0.0,
        }
    }
}

fn fade(shown: Millis, now: Millis) -> f32 {
    let age = now.saturating_sub(shown);
    if age >= HUD_MS {
        return 0.0;
    }
    ((HUD_MS - age) as f32 / FADE_MS as f32).min(1.0)
}

fn pulse(age: Millis) -> [f32; 4] {
    let phase = (age % PULSE_MS) as f32 / PULSE_MS as f32;
    let t = 0.5 + 0.5 * (phase * std::f32::consts::TAU).cos();
    std::array::from_fn(|i| ALARM_DIM[i] + (ALARM_BRIGHT[i] - ALARM_DIM[i]) * t)
}

fn faded(colour: [f32; 4], alpha: f32) -> [f32; 4] {
    [colour[0], colour[1], colour[2], colour[3] * alpha]
}
