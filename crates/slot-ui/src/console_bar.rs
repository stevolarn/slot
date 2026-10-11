//! The bottom bar drawn as the top of the console whose shelf is showing:
//! its plastic, its cart slot, its buttons and badges. The cart drops into
//! the slot between the two layers: the back is the whole bar, the front
//! is the same bar with the space above the slot cut away, so the part of
//! the cart that has gone into the slot is hidden.

use slot_gfx::OUT_W;
use slot_store::Platform;

use crate::art::render_svg;
use crate::plate::UndoFace;
use crate::slot_chrome::{MOUTH_H, MOUTH_W};
use crate::text::{coverage, Layout};

pub const BAR_W: u32 = OUT_W;
pub const BAR_H: u32 = MOUTH_H as u32;

/// The slot, in bar coordinates. A seated cart's top edge rests just
/// inside it.
const SLOT_Y: f32 = 5.0;
const SLOT_H: f32 = 11.0;
/// Where the cart vanishes into the slot.
const SLOT_CUT: u32 = 11;
/// The top of the shelf's name, under the slot.
pub const NAME_Y: f32 = 23.0;

const CX: f32 = BAR_W as f32 / 2.0;
const MOUTH_X: f32 = CX - MOUTH_W / 2.0;

/// The bar's two layers: drawn behind the cart, and in front of it.
pub struct BarFaces {
    pub back: UndoFace,
    pub front: UndoFace,
}

pub fn bar_faces(platform: Platform) -> Option<BarFaces> {
    let mut back = render_svg(&svg(platform), BAR_W, BAR_H)?;
    for t in words(platform) {
        print(&mut back, &t);
    }
    let mut front = back.clone();
    let (x0, x1) = (MOUTH_X.floor() as u32, (MOUTH_X + MOUTH_W).ceil() as u32);
    for y in 0..SLOT_CUT {
        for x in x0..x1.min(BAR_W) {
            front[((y * BAR_W + x) * 4 + 3) as usize] = 0;
        }
    }
    let face = |rgba| UndoFace {
        rgba,
        w: BAR_W,
        h: BAR_H,
    };
    Some(BarFaces {
        back: face(back),
        front: face(front),
    })
}

struct Words {
    text: &'static str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    px: f32,
    tracking: f32,
    ink: [u8; 3],
}

#[allow(clippy::too_many_arguments)]
const fn words_at(
    text: &'static str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    px: f32,
    tracking: f32,
    ink: [u8; 3],
) -> Words {
    Words {
        text,
        x,
        y,
        w,
        h,
        px,
        tracking,
        ink,
    }
}

fn print(rgba: &mut [u8], t: &Words) {
    let (w, h) = (t.w as u32, t.h as u32);
    let layout = Layout {
        lines: vec![t.text.to_string()],
        px: t.px,
        tracking: t.tracking,
    };
    for (i, a) in coverage(w, h, &layout).into_iter().enumerate() {
        if a == 0 {
            continue;
        }
        let (x, y) = (t.x as u32 + i as u32 % w, t.y as u32 + i as u32 / w);
        if x >= BAR_W || y >= BAR_H {
            continue;
        }
        let d = ((y * BAR_W + x) * 4) as usize;
        let a = a as u32;
        for c in 0..3 {
            rgba[d + c] = ((t.ink[c] as u32 * a + rgba[d + c] as u32 * (255 - a)) / 255) as u8;
        }
        rgba[d + 3] = rgba[d + 3].max(a as u8);
    }
}

fn words(platform: Platform) -> Vec<Words> {
    const NES_RED: [u8; 3] = [0xc4, 0x12, 0x2f];
    const WHITE: [u8; 3] = [0xee, 0xee, 0xee];
    const GOLD: [u8; 3] = [0xd9, 0xb4, 0x4a];
    match platform {
        Platform::Nes => vec![
            words_at("Nintendo", 18.0, 6.0, 120.0, 18.0, 17.0, 0.0, NES_RED),
            words_at(
                "ENTERTAINMENT SYSTEM",
                18.0,
                22.0,
                120.0,
                10.0,
                8.0,
                1.0,
                [0x5a, 0x59, 0x58],
            ),
            words_at("POWER", 52.0, 50.0, 46.0, 8.0, 7.0, 1.0, [0xd8, 0xd6, 0xd2]),
            words_at(
                "RESET",
                104.0,
                50.0,
                46.0,
                8.0,
                7.0,
                1.0,
                [0xd8, 0xd6, 0xd2],
            ),
        ],
        Platform::Snes => vec![
            words_at(
                "Nintendo",
                22.0,
                10.0,
                104.0,
                12.0,
                10.0,
                0.0,
                [0x6a, 0x6a, 0x6e],
            ),
            words_at(
                "SUPER NINTENDO",
                14.0,
                24.0,
                120.0,
                14.0,
                12.0,
                1.0,
                [0x4a, 0x4a, 0x50],
            ),
            words_at(
                "ENTERTAINMENT SYSTEM",
                14.0,
                38.0,
                120.0,
                10.0,
                7.0,
                1.0,
                [0x6a, 0x6a, 0x6e],
            ),
            words_at(
                "POWER",
                160.0,
                47.0,
                56.0,
                9.0,
                7.0,
                1.0,
                [0x55, 0x55, 0x5a],
            ),
            words_at(
                "RESET",
                504.0,
                47.0,
                56.0,
                9.0,
                7.0,
                1.0,
                [0x55, 0x55, 0x5a],
            ),
        ],
        Platform::Md => vec![
            words_at(
                "16-BIT",
                556.0,
                24.0,
                64.0,
                22.0,
                13.0,
                1.0,
                [0x10, 0x10, 0x10],
            ),
            words_at("MEGA DRIVE", 616.0, 14.0, 100.0, 12.0, 10.0, 1.0, WHITE),
            words_at(
                "SEGA",
                616.0,
                28.0,
                100.0,
                22.0,
                20.0,
                2.0,
                [0x2f, 0x6c, 0xd6],
            ),
            words_at(
                "HIGH DEFINITION GRAPHICS",
                130.0,
                46.0,
                110.0,
                9.0,
                6.0,
                1.0,
                GOLD,
            ),
        ],
        Platform::Gb => vec![
            words_at("OFF", 22.0, 16.0, 30.0, 10.0, 8.0, 1.0, [0x6a, 0x68, 0x80]),
            words_at("ON", 84.0, 16.0, 30.0, 10.0, 8.0, 1.0, [0x6a, 0x68, 0x80]),
            words_at(
                "Nintendo",
                586.0,
                12.0,
                110.0,
                16.0,
                14.0,
                0.0,
                [0x2c, 0x2f, 0x6b],
            ),
            words_at(
                "GAME BOY",
                586.0,
                28.0,
                110.0,
                18.0,
                15.0,
                1.0,
                [0x2c, 0x2f, 0x6b],
            ),
        ],
        Platform::Gbc => vec![
            words_at("OFF", 22.0, 16.0, 30.0, 10.0, 8.0, 1.0, WHITE),
            words_at("ON", 84.0, 16.0, 30.0, 10.0, 8.0, 1.0, WHITE),
            words_at("GAME BOY", 590.0, 24.0, 110.0, 16.0, 13.0, 1.0, WHITE),
        ],
        Platform::Gba => vec![
            words_at("L", 20.0, 3.0, 40.0, 16.0, 13.0, 0.0, [0x4a, 0x44, 0x7e]),
            words_at("R", 660.0, 3.0, 40.0, 16.0, 13.0, 0.0, [0x4a, 0x44, 0x7e]),
            words_at(
                "POWER",
                612.0,
                28.0,
                60.0,
                10.0,
                7.0,
                1.0,
                [0xb8, 0xb2, 0xe0],
            ),
            words_at(
                "Nintendo",
                34.0,
                30.0,
                100.0,
                14.0,
                11.0,
                0.0,
                [0xb8, 0xb2, 0xe0],
            ),
        ],
    }
}

/// The dark plate under the slot that the shelf's name sits on.
fn name_plate(fill: &str, rim: &str) -> String {
    let (x, w) = (CX - 112.0, 224.0);
    format!(
        r#"<rect x="{x}" y="{y}" width="{w}" height="28" rx="6" fill="{fill}" stroke="{rim}" stroke-width="1.5"/>"#,
        y = NAME_Y - 2.0,
    )
}

/// The slot itself: a dark mouth with a lighter lip under it.
fn slot(rx: f32, inner: &str, lip: &str) -> String {
    format!(
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{inner}"/>
<rect x="{x}" y="{ly}" width="{w}" height="1.5" fill="{lip}"/>"#,
        x = MOUTH_X,
        y = SLOT_Y,
        w = MOUTH_W,
        h = SLOT_H,
        ly = SLOT_Y + SLOT_H,
    )
}

fn ridges(x: f32, y: f32, w: f32, h: f32, step: f32, ink: &str, vertical: bool) -> String {
    let mut s = String::new();
    let n = ((if vertical { w } else { h }) / step) as usize;
    for i in 0..n {
        let o = i as f32 * step + step / 2.0;
        if vertical {
            s += &format!(
                r#"<rect x="{}" y="{y}" width="1" height="{h}" fill="{ink}"/>"#,
                x + o
            );
        } else {
            s += &format!(
                r#"<rect x="{x}" y="{}" width="{w}" height="1" fill="{ink}"/>"#,
                y + o
            );
        }
    }
    s
}

fn svg(platform: Platform) -> String {
    let body = match platform {
        Platform::Nes => nes(),
        Platform::Snes => snes(),
        Platform::Md => md(),
        Platform::Gb => gb(),
        Platform::Gbc => gbc(),
        Platform::Gba => gba(),
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{BAR_W}" height="{BAR_H}" viewBox="0 0 {BAR_W} {BAR_H}">{body}</svg>"#
    )
}

/// The front-loading NES: light grey over a darker grey band, the ribbed
/// black block on the right, the flip-up door across the top.
fn nes() -> String {
    let mut s = String::new();
    s += r##"<defs><linearGradient id="top" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#d4d3d0"/><stop offset="1" stop-color="#c4c3c0"/></linearGradient></defs>
<rect width="720" height="58" fill="#858382"/>
<rect width="720" height="38" fill="url(#top)"/>
<rect y="36" width="720" height="2" fill="#a9a8a6"/>
<rect y="38" width="720" height="1" fill="#6e6c6b"/>
<rect x="150" y="2" width="420" height="31" rx="2" fill="#cfcecb" stroke="#a7a6a3" stroke-width="1.5"/>
<rect x="152" y="31" width="416" height="1" fill="#e2e1de"/>"##;
    s += &slot(1.0, "#1b1b1c", "#e6e5e2");
    s += r##"<rect x="40" y="41" width="122" height="14" rx="2" fill="#6e6c6b"/>
<circle cx="47" cy="48" r="2.4" fill="#e0262d"/>
<rect x="56" y="43" width="40" height="9" rx="1" fill="#3a3a3b"/>
<rect x="108" y="43" width="40" height="9" rx="1" fill="#3a3a3b"/>
<rect x="56" y="43" width="40" height="1" fill="#575758"/>
<rect x="108" y="43" width="40" height="1" fill="#575758"/>
<rect x="636" y="0" width="84" height="58" fill="#2e2e2e"/>
<rect x="636" y="0" width="2" height="58" fill="#1c1c1c"/>"##;
    s += &ridges(640.0, 2.0, 80.0, 56.0, 3.0, "#454545", false);
    s += &ridges(452.0, 41.0, 170.0, 14.0, 3.0, "#757372", true);
    s += &name_plate("#3c3b3b", "#6e6c6b");
    s
}

/// The PAL SNES seen from above: light grey body, the darker cart bay with
/// its long slot, POWER and RESET either side, the four-colour badge.
fn snes() -> String {
    let mut s = String::new();
    s += r##"<defs><linearGradient id="body" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#dcdcde"/><stop offset="1" stop-color="#c3c3c6"/></linearGradient>
<linearGradient id="bay" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#999b9b"/><stop offset="0.15" stop-color="#a5a7a7"/><stop offset="1" stop-color="#aeb0b0"/></linearGradient></defs>
<rect width="720" height="58" fill="url(#body)"/>
<rect y="0" width="720" height="1.5" fill="#ececee"/>
<rect x="146" y="1" width="428" height="54" rx="12" fill="url(#bay)" stroke="#8f9191" stroke-width="1.5"/>"##;
    s += &slot(5.0, "#26262a", "#c4c6c6");
    s += r##"<rect x="160" y="20" width="56" height="26" rx="4" fill="#7e7d7e"/>
<rect x="164" y="23" width="48" height="20" rx="3" fill="#5f5e60"/>
<rect x="166" y="24" width="22" height="18" rx="2" fill="#8d8c8e"/>"##;
    s += &ridges(167.0, 25.0, 20.0, 16.0, 3.0, "#6c6b6d", true);
    s += r##"<rect x="504" y="20" width="56" height="26" rx="4" fill="#7e7d7e"/>
<rect x="508" y="23" width="48" height="20" rx="3" fill="#2a2a2c"/>
<rect x="510" y="24" width="44" height="2" rx="1" fill="#48484b"/>
<circle cx="656" cy="20" r="13" fill="#c9c9cc" stroke="#a9a9ad" stroke-width="1.5"/>
<circle cx="650.5" cy="15.5" r="4.2" fill="#2a5bd7"/>
<circle cx="661.5" cy="15.5" r="4.2" fill="#2ea44f"/>
<circle cx="650.5" cy="24.5" r="4.2" fill="#f3c22b"/>
<circle cx="661.5" cy="24.5" r="4.2" fill="#d82b2b"/>
<rect x="600" y="40" width="112" height="12" rx="3" fill="#b5b5b9"/>
<rect x="608" y="43" width="36" height="6" rx="2" fill="#7c7c80"/>
<rect x="668" y="43" width="36" height="6" rx="2" fill="#7c7c80"/>"##;
    s += &name_plate("#5c5c61", "#8a8a8f");
    s
}

/// The first Mega Drive: glossy black, the raised round disc with the slot
/// cut across it and the gold 16-BIT badge, the ribbed vent on the left.
fn md() -> String {
    let mut s = String::new();
    s += r##"<defs><linearGradient id="body" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#202021"/><stop offset="0.3" stop-color="#121213"/><stop offset="1" stop-color="#0b0b0c"/></linearGradient>
<radialGradient id="disc" cx="0.5" cy="0.25" r="0.75">
<stop offset="0" stop-color="#2a2a2c"/><stop offset="1" stop-color="#121213"/></radialGradient></defs>
<rect width="720" height="58" fill="url(#body)"/>
<rect y="0" width="720" height="1" fill="#2c2c2e"/>
<ellipse cx="360" cy="30" rx="198" ry="44" fill="#0a0a0b"/>
<ellipse cx="360" cy="28" rx="192" ry="40" fill="url(#disc)" stroke="#333336" stroke-width="1.5"/>
<ellipse cx="360" cy="28" rx="170" ry="33" fill="none" stroke="#232325" stroke-width="2"/>"##;
    s += &slot(4.0, "#050505", "#3a3a3d");
    s += r##"<rect x="234" y="6" width="252" height="2" fill="#1a1a1c"/>
<rect x="548" y="22" width="80" height="26" rx="3" fill="#0e0e0f" stroke="#2c2c2e" stroke-width="1"/>
<rect x="554" y="24" width="68" height="22" rx="2" fill="#d9b44a"/>
<rect x="8" y="4" width="112" height="50" rx="3" fill="#0d0d0e"/>"##;
    for i in 0..18 {
        let x = 12.0 + i as f32 * 6.0;
        s += &format!(
            r##"<polygon points="{x},50 {},50 {},8 {},8" fill="#2b2b2c"/>"##,
            x + 2.0,
            x + 8.0,
            x + 6.0
        );
    }
    s += r##"<rect x="128" y="8" width="26" height="34" rx="3" fill="#1d1d1f"/>
<rect x="134" y="12" width="14" height="14" rx="2" fill="#3a3a3d"/>
<circle cx="141" cy="34" r="3" fill="#b0232d"/>"##;
    s += &name_plate("#060607", "#2c2c2e");
    s
}

/// The original Game Boy from the top: warm grey, the OFF/ON switch, and
/// the top of the screen bezel with its magenta and navy lines.
fn gb() -> String {
    let mut s = String::new();
    s += r##"<defs><linearGradient id="body" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#d0ccc7"/><stop offset="1" stop-color="#bdb9b4"/></linearGradient></defs>
<rect width="720" height="58" fill="url(#body)"/>
<rect y="0" width="720" height="1.5" fill="#e0dcd7"/>"##;
    s += &slot(2.0, "#2a2928", "#e4e0db");
    s += r##"<rect x="30" y="4" width="76" height="11" rx="5" fill="#9e9a95"/>
<rect x="34" y="5" width="30" height="9" rx="4" fill="#c9c5c0"/>"##;
    s += &ridges(36.0, 6.0, 26.0, 7.0, 3.0, "#a9a5a0", true);
    s += r##"<path d="M150,58 V32 Q150,22 162,22 H558 Q570,22 570,32 V58 Z" fill="#5d5f6c"/>
<rect x="166" y="31" width="64" height="1.5" fill="#8b1d5e"/>
<rect x="166" y="35" width="64" height="1.5" fill="#2c2f6b"/>
<rect x="490" y="31" width="64" height="1.5" fill="#8b1d5e"/>
<rect x="490" y="35" width="64" height="1.5" fill="#2c2f6b"/>"##;
    s += &name_plate("#5d5f6c", "#5d5f6c");
    s
}

/// The Atomic Purple Game Boy Color: see-through purple with the board
/// and its screws showing, the dark screen bezel below the slot.
fn gbc() -> String {
    let mut s = String::new();
    s += r##"<defs><linearGradient id="body" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#9d8fc2"/><stop offset="1" stop-color="#7b6ca6"/></linearGradient></defs>
<rect width="720" height="58" fill="#2f3a2c"/>
<rect x="12" y="20" width="120" height="30" fill="#3d5a36"/>
<rect x="588" y="6" width="120" height="44" fill="#3d5a36"/>
<rect x="20" y="26" width="40" height="14" fill="#1d1d1f"/>
<rect x="600" y="12" width="26" height="10" fill="#1d1d1f"/>
<circle cx="130" cy="44" r="5" fill="#b8b8bc"/>
<circle cx="590" cy="44" r="5" fill="#b8b8bc"/>
<rect width="720" height="58" fill="url(#body)" fill-opacity="0.72"/>
<rect y="0" width="720" height="1.5" fill="#c7bce6"/>"##;
    s += &slot(2.0, "#241d36", "#c7bce6");
    s += r##"<rect x="30" y="4" width="76" height="11" rx="5" fill="#5f5287" fill-opacity="0.85"/>
<rect x="34" y="5" width="30" height="9" rx="4" fill="#b4a8d6"/>
<path d="M150,58 V32 Q150,22 162,22 H558 Q570,22 570,32 V58 Z" fill="#26262c"/>"##;
    s += &name_plate("#26262c", "#26262c");
    s
}

/// The indigo Game Boy Advance: the L and R shoulders at either end, the
/// slot between them, the black screen surround below it.
fn gba() -> String {
    let mut s = String::new();
    s += r##"<defs><linearGradient id="body" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#5a4fb0"/><stop offset="1" stop-color="#40368a"/></linearGradient>
<linearGradient id="shoulder" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#a7a2cc"/><stop offset="1" stop-color="#8580b2"/></linearGradient></defs>
<rect width="720" height="58" fill="url(#body)"/>
<rect y="0" width="720" height="1.5" fill="#7a70cc"/>
<path d="M0,2 H76 Q86,2 88,12 L90,22 H0 Z" fill="url(#shoulder)"/>
<path d="M720,2 H644 Q634,2 632,12 L630,22 H720 Z" fill="url(#shoulder)"/>"##;
    s += &slot(2.0, "#17142e", "#7a70cc");
    s += r##"<path d="M156,58 V30 Q156,20 168,20 H552 Q564,20 564,30 V58 Z" fill="#18171f"/>
<circle cx="604" cy="33" r="3" fill="#3ee06a"/>"##;
    s += &ridges(600.0, 42.0, 100.0, 14.0, 4.0, "#2e2668", true);
    s += &name_plate("#18171f", "#18171f");
    s
}
