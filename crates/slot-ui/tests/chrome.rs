use slot_store::{Cart, Platform};
use slot_ui::{
    cart_box, foot_y, icon_box, seated_box, stick_out, Draw, Shelf, SlotChrome, TexId, ALERT_PX,
    CART_H, CART_W, MOUTH_H, OUT_H, OUT_W,
};

const CENTRED: f32 = (OUT_W - CART_W) as f32 / 2.0;

fn centred(c: &Cart) -> f32 {
    (OUT_W - cart_box(c.platform).0) as f32 / 2.0
}

fn cart() -> Cart {
    Cart {
        platform: Platform::Gba,
        stem: "Emerald".into(),
        rom: "Games/GBA/Emerald.gba".into(),
        label: None,
        code: String::new(),
        shell: None,
        title: "POKEMON EMER".into(),
    }
}

fn pak() -> Cart {
    Cart {
        platform: Platform::Gb,
        stem: "Tetris".into(),
        rom: "Games/GB/Tetris.gb".into(),
        label: None,
        code: String::new(),
        shell: None,
        title: "TETRIS".into(),
    }
}

fn both() -> [(&'static str, Cart); 2] {
    [("the GBA cart", cart()), ("the Game Boy pak", pak())]
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Quad {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn quad(d: &Draw) -> Quad {
    match *d {
        Draw::Rect { x, y, w, h, .. }
        | Draw::Tex { x, y, w, h, .. }
        | Draw::Turned { x, y, w, h, .. } => Quad { x, y, w, h },
        Draw::Game | Draw::Shot { .. } => Quad {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: OUT_H as f32,
        },
    }
}

fn is_cart(d: &Draw) -> bool {
    let cover = matches!(*d, Draw::Rect { colour, .. } if colour == [0.0, 0.0, 0.0, 1.0]);
    !(is_mouth(d) || is_lip(d) || is_housing(d) || is_recess(d) || cover)
        && quad(d).w < OUT_W as f32
        && quad(d).h > 4.0
}

fn is_game_layer(d: &Draw) -> bool {
    matches!(d, Draw::Game)
}

fn is_lip(d: &Draw) -> bool {
    slot_tinted(d, 3)
}

/// One of the four slot colours, in the theme or in any console's own.
fn slot_tinted(d: &Draw, which: usize) -> bool {
    std::iter::once(None)
        .chain(Platform::ALL.into_iter().map(Some))
        .any(|p| tinted(d, slot_ui::slot_colours(p)[which]))
}

fn tinted(d: &Draw, c: [f32; 4]) -> bool {
    match *d {
        Draw::Rect { colour, .. } => (0..3).all(|i| (colour[i] - c[i]).abs() < 0.001),
        _ => false,
    }
}

fn is_mouth(d: &Draw) -> bool {
    slot_tinted(d, 2)
}

fn is_housing(d: &Draw) -> bool {
    slot_tinted(d, 0)
}

fn is_recess(d: &Draw) -> bool {
    slot_tinted(d, 1)
}

fn as_mouth(d: &Draw) -> Option<Quad> {
    is_mouth(d).then(|| quad(d))
}

fn as_lip(d: &Draw) -> Option<Quad> {
    is_lip(d).then(|| quad(d))
}

fn alpha(d: &Draw) -> f32 {
    match *d {
        Draw::Rect { colour, .. } => colour[3],
        Draw::Tex { alpha, .. } | Draw::Turned { alpha, .. } => alpha,
        Draw::Game | Draw::Shot { .. } => 1.0,
    }
}

fn opaque(d: &Draw) -> bool {
    alpha(d) >= 1.0
}

fn cart_at(out: &[Draw]) -> usize {
    out.iter()
        .position(is_cart)
        .expect("no cart sized quad in the list")
}

fn cart_visible_height(out: &[Draw]) -> f32 {
    let i = cart_at(out);
    let cart = quad(&out[i]);
    let lid = out[i + 1..]
        .iter()
        .filter(|d| opaque(d))
        .map(|d| quad(d).y)
        .fold(f32::INFINITY, f32::min);
    (cart.y + cart.h).min(lid).max(cart.y) - cart.y
}

fn chrome_into(c: &Cart, seat: f32, out: &mut Vec<Draw>) {
    SlotChrome {
        cart: c,
        face: None,
        rest: centred(c),
        scale: 1.0,
        seat,
        alert: None,
        dim: 0.5,
        screen: 0.0,
        game: true,
    }
    .draw(out);
}

#[test]
fn a_cart_the_row_had_not_finished_moving_slides_across_as_it_goes_in() {
    let c = cart();
    let mut shelf = Shelf::new(vec![cart(), pak(), cart()]);
    shelf.right();
    shelf.update(1.0 / 60.0);
    shelf.update(1.0 / 60.0);
    let (rest, scale) = shelf.selected_at();
    assert!(
        (rest - CENTRED).abs() > 100.0 && scale < 0.95,
        "the row settled before the press: this proves nothing at {rest} and {scale}"
    );
    let face = TexId::from_raw(3);
    let at = |seat: f32| {
        let mut out = Vec::new();
        SlotChrome {
            cart: &c,
            face: Some(face),
            rest,
            scale,
            seat,
            alert: None,
            dim: 0.0,
            screen: 0.0,
            game: false,
        }
        .draw(&mut out);
        out.iter()
            .find(|d| matches!(d, Draw::Tex { .. }))
            .map(quad)
            .expect("no cartridge in the list")
    };
    let start = at(0.0);
    assert!(
        (start.x - rest).abs() < 0.01 && (start.w - CART_W as f32 * scale).abs() < 0.01,
        "the cart starts at {start:?} rather than in the quad the row had it in"
    );
    assert!(
        (start.y + start.h - foot_y(CART_H as f32)).abs() < 0.01,
        "the cart starts with its foot at {} rather than on the row's floor",
        start.y + start.h
    );
    let seated = at(1.0);
    let sw = seated_box(c.platform).0 as f32;
    let mid = OUT_W as f32 / 2.0;
    assert!(
        (seated.x + seated.w / 2.0 - mid).abs() < 0.01 && (seated.w - sw).abs() < 0.01,
        "the cart seats at {seated:?} rather than the slot's size in the mouth"
    );
    let off = |q: Quad| (q.x + q.w / 2.0 - mid).abs();
    let mut last = start;
    for step in 1..=20 {
        let q = at(step as f32 / 20.0);
        assert!(
            off(q) <= off(last) + 0.01,
            "the cart went back to {} from {}",
            q.x,
            last.x
        );
        assert!(
            (q.w - sw).abs() <= (last.w - sw).abs() + 0.01,
            "the cart moved away from the slot's size, to {} from {}",
            q.w,
            last.w
        );
        last = q;
    }
}

fn draw_powering_on(t: f32, out: &mut Vec<Draw>) {
    let c = cart();
    SlotChrome {
        cart: &c,
        face: None,
        rest: CENTRED,
        scale: 1.0,
        seat: 1.0,
        alert: None,
        dim: 0.0,
        screen: t,
        game: true,
    }
    .draw(out);
}

fn bands(t: f32) -> Vec<Draw> {
    let mut out = Vec::new();
    draw_powering_on(t, &mut out);
    out.into_iter()
        .filter(|d| is_lip(d) || is_mouth(d) || is_housing(d))
        .collect()
}

fn chrome(c: &Cart, seat: f32) -> Vec<Draw> {
    let mut out = Vec::new();
    chrome_into(c, seat, &mut out);
    out
}

fn draw_inserting(c: &Cart, t: f32, out: &mut Vec<Draw>) {
    chrome_into(c, t, out);
}

fn draw_ejecting(c: &Cart, t: f32, out: &mut Vec<Draw>) {
    chrome_into(c, 1.0 - t, out);
}

fn cart_quad(c: &Cart, t: f32) -> Quad {
    let out = chrome(c, t.min(0.999));
    quad(&out[cart_at(&out)])
}

fn cart_y(c: &Cart, t: f32) -> f32 {
    cart_quad(c, t).y
}

fn visible_cart_height(c: &Cart, t: f32) -> f32 {
    let mut out = Vec::new();
    draw_inserting(c, t, &mut out);
    cart_visible_height(&out)
}

#[test]
fn the_slot_is_at_the_bottom_of_the_screen() {
    let mut out = Vec::new();
    draw_inserting(&cart(), 0.5, &mut out);
    let mouth = out.iter().find_map(as_mouth).expect("no mouth drawn");
    assert!(
        mouth.y >= OUT_H as f32 - MOUTH_H - 1.0,
        "the mouth is not on the bottom edge"
    );
}

#[test]
fn the_cart_travels_downward() {
    for (name, c) in both() {
        let early = cart_y(&c, 0.1);
        let late = cart_y(&c, 0.9);
        assert!(late > early, "{name} is going up, not down into the slot");
    }
}

#[test]
fn the_cart_is_progressively_occluded_by_the_lip() {
    for (name, c) in both() {
        let h = |t: f32| visible_cart_height(&c, t);
        assert!(h(0.5) < h(0.1), "{name} is not sinking behind the lip");
        assert!(h(0.95) < h(0.5) * 0.5, "{name} is barely in by the end");
    }
}

#[test]
fn the_lip_is_the_frontmost_band() {
    let mut out = Vec::new();
    draw_inserting(&cart(), 0.5, &mut out);
    let lip = out.iter().rposition(is_lip).unwrap();
    let cart = out.iter().position(is_cart).unwrap();
    let housing = out.iter().rposition(is_housing).unwrap();
    assert!(lip > cart && lip > housing, "the lip is not in front");
}

#[test]
fn the_cart_catches_on_the_lip_before_going_in() {
    for (name, c) in both() {
        let step = |a: f32, b: f32| cart_y(&c, b) - cart_y(&c, a);
        let approach = step(0.15, 0.30);
        let catching = step(0.45, 0.60);
        let through = step(0.70, 0.85);
        assert!(
            catching < approach * 0.5,
            "{name}: no hesitation at the lip"
        );
        assert!(through > catching * 1.5, "{name}: it never pushes through");
    }
}

#[test]
fn an_unseated_cart_stands_where_the_shelf_left_it() {
    for (name, c) in both() {
        let mut shelf = Vec::new();
        Shelf::new(vec![c.clone()]).draw_row(None, 0.0, 0.0, 1.0, &mut shelf);
        let on_shelf = quad(&shelf[cart_at(&shelf)]);

        let out = chrome(&c, 0.0);
        let in_slot = quad(&out[cart_at(&out)]);
        assert_eq!(
            on_shelf, in_slot,
            "{name} jumps from {on_shelf:?} to {in_slot:?} on insert"
        );
    }
}

#[test]
fn a_cart_shrinks_to_the_slot_by_the_lip() {
    let lip = OUT_H as f32 - MOUTH_H;
    for (name, c) in both() {
        let (w, h) = cart_box(c.platform);
        let (sw, sh) = seated_box(c.platform);
        let start = cart_quad(&c, 0.0);
        assert!(
            (start.w - w as f32).abs() < 0.01 && (start.h - h as f32).abs() < 0.01,
            "{name} leaves the shelf at {}x{} rather than its own {w}x{h}",
            start.w,
            start.h
        );
        let mut last = start;
        for step in 1..=400 {
            let t = step as f32 / 400.0;
            let q = cart_quad(&c, t);
            assert!(q.w <= last.w + 0.01, "{name} grew to {} at seat {t}", q.w);
            if q.y + q.h >= lip {
                assert!(
                    (q.w - sw as f32).abs() < 0.01 && (q.h - sh as f32).abs() < 0.01,
                    "{name} is {}x{} at the lip, not the slot's {sw}x{sh}",
                    q.w,
                    q.h
                );
            }
            last = q;
        }
    }
}

#[test]
fn every_cartridge_stands_out_of_the_slot_as_far_as_its_console_holds_it() {
    let lip = OUT_H as f32 - MOUTH_H;
    let mut shown = Vec::new();
    for p in Platform::ALL {
        let c = Cart {
            platform: p,
            ..cart()
        };
        let out = chrome(&c, 1.0);
        let top = quad(&out[cart_at(&out)]).y;
        let sh = seated_box(p).1 as f32;
        let out_of_slot = (lip - top).max(0.0);
        assert!(
            (out_of_slot - stick_out(p) * sh).abs() < 7.0,
            "{p:?} stands {out_of_slot}px out of a {sh}px seated cart, not {}",
            stick_out(p) * sh
        );
        shown.push((p, stick_out(p)));
    }
    let share = |p| shown.iter().find(|(q, _)| *q == p).unwrap().1;
    assert!(
        share(Platform::Md) > share(Platform::Snes),
        "a Mega Drive cart sits deeper than a SNES one"
    );
    assert!(
        share(Platform::Snes) > share(Platform::Gb),
        "a SNES cart sits deeper than a Game Boy one"
    );
    assert!(
        share(Platform::Gb) > share(Platform::Gba),
        "a Game Boy cart sits deeper than a GBA one"
    );
    assert_eq!(
        share(Platform::Nes),
        0.0,
        "an NES cart is left showing out of the front loader"
    );
}

#[test]
fn every_cartridge_lands_its_foot_on_the_lip_at_the_same_moment() {
    let lip = OUT_H as f32 - MOUTH_H;
    let landings: Vec<(&str, f32, f32)> = both()
        .iter()
        .map(|(name, c)| {
            let foot = |t: f32| {
                let q = cart_quad(c, t);
                q.y + q.h
            };
            let at = (0..=400)
                .map(|s| s as f32 / 400.0)
                .find(|t| foot(*t) >= lip)
                .unwrap_or_else(|| panic!("{name} never reaches the lip at all"));
            (*name, at, foot(at))
        })
        .collect();
    for (name, at, landed) in &landings {
        assert!(
            (landed - lip).abs() < 1.0,
            "{name} is at {landed} on the frame it passes a lip at {lip}: it went through it \
             rather than onto it"
        );
        assert!(
            *at > 0.0,
            "{name} is already on the lip before the travel starts"
        );
    }
    let (first, rest) = landings.split_first().expect("two cartridges");
    for (name, at, _) in rest {
        assert!(
            (at - first.1).abs() < 0.01,
            "{name} lands on the lip at seat {at} and {} at {}: the catch is not the same \
             moment for both",
            first.0,
            first.1
        );
    }
}

#[test]
fn a_seated_cart_stops_in_the_opening_and_covers_its_base() {
    for (name, c) in both() {
        let out = chrome(&c, 1.0);
        let cart = quad(&out[cart_at(&out)]);
        let recess = out
            .iter()
            .find_map(|d| is_recess(d).then(|| quad(d)))
            .expect("no recess drawn");
        assert!(
            cart.y + cart.h > recess.y + recess.h,
            "{name} does not reach the bottom of the recess"
        );
    }
}

#[test]
fn the_cart_is_never_the_frontmost_thing() {
    let out = chrome(&cart(), 0.6);
    let cart = cart_at(&out);
    assert!(
        cart + 1 < out.len(),
        "the cart is the last thing drawn and would sit on top of the case"
    );
}

#[test]
fn ejecting_reverses_the_travel() {
    for (name, c) in both() {
        let mut a = Vec::new();
        draw_ejecting(&c, 0.1, &mut a);
        let mut b = Vec::new();
        draw_ejecting(&c, 0.9, &mut b);
        assert!(
            cart_visible_height(&b) > cart_visible_height(&a),
            "{name} is not coming out"
        );
    }
}

#[test]
fn the_seated_cart_leaves_with_the_case_not_through_it() {
    for t in [0.2, 0.4, 0.6, 0.8] {
        let mut out = Vec::new();
        draw_powering_on(t, &mut out);
        let cart = alpha(&out[cart_at(&out)]);
        let case = out
            .iter()
            .find_map(|d| is_housing(d).then(|| alpha(d)))
            .expect("no case drawn");
        assert!(
            (cart - case).abs() < 0.001,
            "at {t} the cart is at {cart} and the case at {case}: it is not leaving with it"
        );
        assert!(out.iter().any(is_game_layer), "no picture at {t}");
    }
}

#[test]
fn a_dark_screen_lists_no_game_layer() {
    let mut out = Vec::new();
    draw_inserting(&cart(), 0.5, &mut out);
    assert!(
        !out.iter().any(is_game_layer),
        "the picture is drawn with the screen off"
    );
}

#[test]
fn the_chrome_does_not_scale_with_the_screen() {
    let mut out = Vec::new();
    draw_powering_on(0.3, &mut out);
    let lip = out.iter().find_map(as_lip).expect("no lip");
    let mut settled = Vec::new();
    draw_powering_on(1.0, &mut settled);
    let lip2 = settled.iter().find_map(as_lip).unwrap();
    assert_eq!(
        (lip.y, lip.h),
        (lip2.y, lip2.h),
        "the lip moved with the picture"
    );
}

#[test]
fn the_slot_is_solid_until_the_picture_is_behind_it() {
    let dark = bands(0.0);
    assert!(dark.len() >= 3, "the slot lost its bands");
    assert!(
        dark.iter().all(opaque),
        "the backdrop shows through the slot"
    );
    assert!(
        bands(1.0).iter().all(|d| alpha(d) == 0.0),
        "the slot is still painted over the picture"
    );
}

#[test]
fn the_seated_cart_never_shows_through_the_fading_housing() {
    for t in [0.25, 0.5, 0.75] {
        let mut out = Vec::new();
        draw_powering_on(t, &mut out);
        let at = cart_at(&out);
        let cart = quad(&out[at]);
        let overlaps = |q: Quad| {
            q.x < cart.x + cart.w
                && cart.x < q.x + q.w
                && q.y < cart.y + cart.h
                && cart.y < q.y + q.h
        };
        let black =
            |d: &Draw| matches!(*d, Draw::Rect { colour, .. } if colour == [0.0, 0.0, 0.0, 1.0]);
        for (i, d) in out.iter().enumerate().skip(at + 1) {
            if !(is_housing(d) || is_lip(d)) || alpha(d) >= 1.0 || !overlaps(quad(d)) {
                continue;
            }
            assert!(
                out[at + 1..i]
                    .iter()
                    .any(|b| black(b) && quad(b) == quad(d)),
                "at {t} the cart shows through {:?}",
                quad(d)
            );
        }
    }
}

#[test]
fn the_alert_fits_on_the_cart_face() {
    let (w, h) = icon_box(ALERT_PX);
    let (cw, ch) = seated_box(Platform::Gba);
    assert!(w < cw && h < ch, "a {w}x{h} alert on a {cw}x{ch} cart");
    assert!(h * 4 > ch, "a {h} px alert on a {ch} px cart is a speck");
}

#[test]
fn the_travel_eases_out_into_the_seat() {
    for (name, c) in both() {
        let last = (cart_y(&c, 1.0) - cart_y(&c, 0.9)).abs();
        let through = (cart_y(&c, 0.85) - cart_y(&c, 0.70)).abs() / 1.5;
        assert!(
            last < through * 0.5,
            "{name} covers {last} px in the last tenth against {through} while dropping \
             through: it arrives at speed and stops dead"
        );
    }
}

#[test]
fn the_empty_slot_is_the_same_slot_the_chrome_draws() {
    let c = cart();
    let mut empty = Vec::new();
    slot_ui::draw_empty_slot_for(Some(c.platform), &mut empty);

    let mut seated = Vec::new();
    SlotChrome {
        cart: &c,
        face: None,
        rest: CENTRED,
        scale: 1.0,
        seat: 1.0,
        alert: None,
        dim: 0.0,
        screen: 0.0,
        game: false,
    }
    .draw(&mut seated);

    let slot_of = |list: &[Draw]| -> Vec<(Quad, f32)> {
        list.iter()
            .filter(|d| is_housing(d) || is_lip(d) || is_mouth(d) || is_recess(d))
            .map(|d| (quad(d), alpha(d)))
            .collect()
    };
    assert_eq!(
        slot_of(&empty),
        slot_of(&seated),
        "the slot is not the same object on the two screens"
    );
    assert!(
        empty.iter().any(is_recess),
        "the empty slot has no recess, so it is a hole onto the backdrop"
    );
}

#[test]
fn nothing_is_ever_ruled_across_the_cart() {
    for (_, c) in both() {
        for step in 0..=20 {
            let seat = step as f32 / 20.0;
            let out = chrome(&c, seat);
            let cart = out.iter().position(is_cart).expect("no cart drawn");
            for (i, d) in out.iter().enumerate() {
                if is_mouth(d) || is_recess(d) {
                    assert!(
                        i < cart,
                        "at {seat} a hole at {i} is over the cart at {cart}"
                    );
                }
            }
            assert!(
                out[..cart].iter().filter(|d| is_mouth(d)).count() > 4,
                "the scoop is not part of the hole"
            );

            let c = quad(&out[cart]);
            let mut x = c.x + 1.0;
            while x < c.x + c.w {
                let mut cover: Vec<(f32, f32)> = out[cart + 1..]
                    .iter()
                    .filter(|d| opaque(d))
                    .map(quad)
                    .filter(|q| q.x <= x && q.x + q.w > x)
                    .map(|q| (q.y, q.y + q.h))
                    .collect();
                cover.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                let mut run: Option<(f32, f32)> = None;
                for (top, bottom) in cover {
                    run = Some(match run {
                        None => (top, bottom),
                        Some((t, b)) => {
                            assert!(
                                top <= b + 0.01,
                                "at {seat}, x {x}: cover breaks at {b} and resumes at {top}, so \
                             something is ruled across the cart"
                            );
                            (t, b.max(bottom))
                        }
                    });
                }
                if let Some((_, bottom)) = run {
                    assert!(
                        bottom >= OUT_H as f32 - 0.01,
                        "at {seat}, x {x}: cover stops at {bottom}, short of the bottom"
                    );
                }
                x += 4.0;
            }

            let recess = out[..cart]
                .iter()
                .find_map(|d| is_recess(d).then(|| quad(d)))
                .expect("no recess drawn");
            for d in &out[cart + 1..] {
                let q = quad(d);
                let inside = q.y > recess.y + 0.01 && q.y + q.h < recess.y + recess.h - 0.01;
                let over_cart = q.x < c.x + c.w && q.x + q.w > c.x;
                let band = q.w > c.w / 2.0;
                assert!(
                    !(opaque(d) && inside && over_cart && band),
                    "at {seat}, {q:?} cuts a band out of the cart inside the recess"
                );
            }
        }
    }
}

#[test]
fn the_slot_has_a_thumb_scoop_across_its_middle() {
    let out = chrome(&cart(), 0.5);
    let dark: Vec<_> = out.iter().filter(|d| is_mouth(d)).map(quad).collect();
    let slit = dark
        .iter()
        .copied()
        .max_by(|a, b| a.w.partial_cmp(&b.w).unwrap())
        .expect("no slot drawn");
    let below: Vec<_> = dark
        .iter()
        .copied()
        .filter(|q| q.y >= slit.y + slit.h)
        .collect();
    assert!(below.len() >= 8, "the scoop is not cut as a curve");

    let span_of = |pick: &dyn Fn(&Quad) -> bool| -> (f32, f32) {
        let l = below
            .iter()
            .filter(|q| pick(q))
            .map(|q| q.x)
            .fold(f32::INFINITY, f32::min);
        let r = below
            .iter()
            .filter(|q| pick(q))
            .map(|q| q.x + q.w)
            .fold(f32::NEG_INFINITY, f32::max);
        (l, r)
    };
    let (l, r) = span_of(&|_| true);
    let centre = OUT_W as f32 / 2.0;
    assert!(
        ((l + r) / 2.0 - centre).abs() < 1.0,
        "the scoop runs {l}..{r} and is not centred"
    );
    assert!(r - l < slit.w, "the scoop is as wide as the opening itself");

    let deepest = below.iter().map(|q| q.y + q.h).fold(0.0, f32::max);
    let (dl, dr) = span_of(&|q| q.y + q.h > deepest - 0.5);
    assert!(
        dr - dl < (r - l) * 0.6,
        "the scoop is a rectangle, not an arc: {}px across at its deepest against {}px at \
         the top",
        dr - dl,
        r - l
    );
    assert!(
        ((dl + dr) / 2.0 - centre).abs() < 1.0,
        "the deepest part of the scoop is off to one side"
    );

    let mut edges: Vec<(f32, f32)> = below.iter().map(|q| (q.x, q.y + q.h)).collect();
    edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for pair in edges.windows(2) {
        let step = (pair[1].1 - pair[0].1).abs();
        assert!(
            step <= 1.01,
            "the curve steps {step}px at x {}: that reads as a jag",
            pair[1].0
        );
    }

    assert!(
        out.iter().any(|d| is_lip(d) && quad(d).y > slit.y),
        "the scoop has no lit rim, so the plastic has no edge"
    );
}

#[test]
fn the_slot_sits_in_a_recessed_bay() {
    let out = chrome(&cart(), 0.5);
    let bay = out
        .iter()
        .find_map(|d| is_recess(d).then(|| quad(d)))
        .expect("no bay drawn");
    let slit = out
        .iter()
        .filter(|d| is_mouth(d))
        .map(quad)
        .max_by(|a, b| a.w.partial_cmp(&b.w).unwrap())
        .expect("no slot drawn");
    assert!(bay.w > slit.w, "the bay is no wider than the opening in it");
    assert!(
        bay.x < slit.x && bay.x + bay.w > slit.x + slit.w,
        "the opening is not inside the bay"
    );
    let step = |a: [f32; 4], b: [f32; 4]| (0..3).map(|i| a[i] - b[i]).sum::<f32>();
    for p in std::iter::once(None).chain(Platform::ALL.into_iter().map(Some)) {
        let [housing, recess, opening, _] = slot_ui::slot_colours(p);
        assert!(
            step(housing, recess) > 0.0 && step(recess, opening) > 0.0,
            "{p:?}: the bay does not read as a step between the shell and the opening"
        );
    }
}

#[test]
fn the_cart_catches_on_the_top_edge_of_the_slot() {
    let lip = OUT_H as f32 - MOUTH_H;
    for (name, c) in both() {
        let bottom_at = |t: f32| {
            let out = chrome(&c, t);
            match out[cart_at(&out)] {
                Draw::Rect { y, h, .. } | Draw::Tex { y, h, .. } => y + h,
                ref other => panic!("the cart is not a quad: {other:?}"),
            }
        };
        let mut slowest = (f32::MAX, 0.0f32);
        let mut t = 0.05;
        while t < 0.95 {
            let d = bottom_at(t + 0.05) - bottom_at(t);
            if d < slowest.0 {
                slowest = (d, t);
            }
            t += 0.05;
        }
        let at = bottom_at(slowest.1);
        assert!(
            (at - lip).abs() < 16.0,
            "{name} hesitates at {at} but the slot's top edge is {lip}"
        );
    }
}
