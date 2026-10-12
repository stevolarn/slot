use std::time::Instant;

use slot_gfx::{Compositor, Draw, TexId, OUT_H, OUT_W};
use slot_input::{InputSource, Millis};
use slot_power::{Platform, Power};
use slot_ui::{
    cart_shadow, console_cart_shadow, date_time_text, game_name_face, gb_cart_shadow, hhmm,
    icon_face, menu_face, quick_caret_face, quick_label_face, quick_legend_faces, quick_value_face,
    rebuild_count_face, rebuild_title_face, set_clock_hint_face, sticker_face, toast_face,
    wallpaper_face, word_face, CartFace, Console, GbShell, Icon, PowerChoice, QuickMenuFaces,
    QuickRow, QuickValue, RebuildScreen, StickerFields, Toast, UndoFace, BOLT_PX, HUD_ICON_PX,
    HUD_INK,
};

use crate::app::{App, Phase};
use crate::build_info::Build;
use crate::cart_faces::CartFaces;
use crate::label_cache::{self, Rebuild};
use crate::session::Session;
use crate::wallpaper;

pub struct Frontend {
    session: Session,
    start: Instant,
    last: Instant,
    draws: Vec<Draw>,
    game_name_tex: Option<TexId>,
    game_named: Option<String>,
    box_tex: Option<TexId>,
    boxed: Option<std::path::PathBuf>,
    box_back_tex: Option<TexId>,
    backed: Option<std::path::PathBuf>,
    box_loading: Option<BoxLoad>,
    back_loading: Option<BoxLoad>,
    box_preloaded_for: Option<std::path::PathBuf>,
    cart_faces: CartFaces,
    rebuild: Option<Rebuild>,
    stale_check: Option<std::sync::mpsc::Receiver<Vec<slot_store::Cart>>>,
    ui_faces: Option<std::sync::mpsc::Receiver<UiFaces>>,
    rebuild_faces: RebuildFaces,
    clocks: Clocks,
    about: AboutFace,
    quick_clock: QuickClock,
}

#[derive(Default)]
struct RebuildFaces {
    title: Option<(TexId, u32, u32)>,
    count: Option<TexId>,
    count_size: (u32, u32),
    shown: Option<usize>,
}

#[derive(Default)]
struct QuickClock {
    dim: Option<TexId>,
    lit: Option<TexId>,
    shown: String,
}

#[derive(Default)]
struct AboutFace {
    tex: Option<TexId>,
    battery: Option<u8>,
}

#[derive(Default)]
struct Clocks {
    line: Option<TexId>,
    hint: Option<TexId>,
    shelf: Option<TexId>,
    picked: Option<String>,
    shown: String,
    battery: String,
    battery_tex: Option<TexId>,
    platform: String,
    platform_tex: Option<TexId>,
}

impl Frontend {
    pub fn boot(platform: Box<dyn Platform>) -> Self {
        Self::boot_with(platform, crate::audio::opened_sink())
    }

    pub fn boot_with(platform: Box<dyn Platform>, sink: Box<dyn crate::audio::AudioSink>) -> Self {
        let now = Instant::now();
        let cart_faces = CartFaces::spawn(platform.root().to_path_buf());
        let mut session = Session::boot_with(platform.root().to_path_buf(), sink);
        crate::boot_time::mark("session");
        let stale_check =
            Self::check_labels(platform.root(), session.app().carts().cloned().collect());
        session.app_mut().set_power(Power::new(platform));
        Frontend {
            session,
            start: now,
            last: now,
            draws: Vec::new(),
            game_name_tex: None,
            game_named: None,
            box_tex: None,
            boxed: None,
            box_back_tex: None,
            backed: None,
            box_loading: None,
            back_loading: None,
            box_preloaded_for: None,
            cart_faces,
            rebuild: None,
            stale_check,
            ui_faces: None,
            rebuild_faces: RebuildFaces::default(),
            clocks: Clocks::default(),
            about: AboutFace::default(),
            quick_clock: QuickClock::default(),
        }
    }

    pub fn upload_faces(&mut self, compositor: &mut Compositor) {
        self.upload_boot_faces(compositor);
        if let Some(faces) = self.ui_faces.take().and_then(|rx| rx.recv().ok()) {
            self.place_ui_faces(compositor, faces);
        }
    }

    pub fn upload_boot_faces(&mut self, compositor: &mut Compositor) {
        if self.rebuild.is_none() {
            self.cart_faces
                .sync(self.session.app_mut(), compositor, true);
        }
        crate::boot_time::mark("cart faces");
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("slot-ui-faces".into())
            .spawn(move || {
                let _ = tx.send(UiFaces::build());
            });
        match spawned {
            Ok(_) => self.ui_faces = Some(rx),
            Err(_) => self.place_ui_faces(compositor, UiFaces::build()),
        }
        let icons = Icon::ALL
            .iter()
            .map(|i| {
                let f = icon_face(*i, HUD_ICON_PX, HUD_INK);
                compositor.create_texture(f.w, f.h, &f.rgba)
            })
            .collect();
        self.session.app_mut().set_icon_faces(icons);
        let hint = set_clock_hint_face();
        self.clocks.hint = Some(compositor.create_texture(hint.w, hint.h, &hint.rgba));
        let shadow = cart_shadow();
        let id = compositor.create_texture(shadow.w, shadow.h, &shadow.rgba);
        self.session.app_mut().set_cart_shadow(id);
        let notched = gb_cart_shadow(GbShell::Notched);
        let notched = compositor.create_texture(notched.w, notched.h, &notched.rgba);
        let rounded = gb_cart_shadow(GbShell::Rounded);
        let rounded = compositor.create_texture(rounded.w, rounded.h, &rounded.rgba);
        self.session.app_mut().set_gb_cart_shadows(notched, rounded);
        for console in [Console::Nes, Console::Md, Console::Snes] {
            let shadow = console_cart_shadow(console);
            let id = compositor.create_texture(shadow.w, shadow.h, &shadow.rgba);
            self.session.app_mut().set_console_cart_shadow(console, id);
        }
        let bolt = icon_face(Icon::Charging, BOLT_PX, HUD_INK);
        let bolt_id = compositor.create_texture(bolt.w, bolt.h, &bolt.rgba);
        self.session.app_mut().set_bolt_face(bolt_id);
        crate::boot_time::mark("shelf faces");
        self.upload_wallpaper(compositor);
    }

    fn poll_ui_faces(&mut self, compositor: &mut Compositor) {
        let Some(rx) = &self.ui_faces else {
            return;
        };
        match rx.try_recv() {
            Ok(faces) => {
                self.ui_faces = None;
                self.place_ui_faces(compositor, faces);
                crate::boot_time::mark("ui faces");
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.ui_faces = None,
        }
    }

    fn place_ui_faces(&mut self, compositor: &mut Compositor, faces: UiFaces) {
        let mut cart = |f: CartFace| compositor.create_texture(f.w, f.h, &f.rgba);
        let toasts = faces.toasts.into_iter().map(&mut cart).collect();
        let mut up = |f: UndoFace| (compositor.create_texture(f.w, f.h, &f.rgba), f.w, f.h);
        let lines = faces.shutdown.into_iter().map(&mut up).collect();
        let menu = faces.power_menu.into_iter().map(&mut up).collect();
        let labels = faces.quick_labels.into_iter().map(&mut up).collect();
        let values = faces
            .quick_values
            .into_iter()
            .map(|v| v.map(&mut up))
            .collect();
        let carets = faces.quick_carets.map(&mut up);
        let legend = faces.quick_legend.map(|f| {
            let (tex, w, _) = up(f);
            (tex, w)
        });
        let help = faces
            .help
            .into_iter()
            .map(|f| {
                let (tex, w, _) = up(f);
                (tex, w)
            })
            .collect();
        let app = self.session.app_mut();
        app.set_shutdown_faces(lines);
        app.set_power_menu_faces(menu);
        app.set_help_faces(help);
        app.set_quick_menu_faces(QuickMenuFaces {
            labels,
            values,
            carets,
            legend,
        });
        app.set_toast_faces(toasts);
    }

    fn upload_wallpaper(&mut self, compositor: &mut Compositor) {
        let app = self.session.app();
        let seed = app.wall_secs().unsigned_abs();
        let Some(rgba) = app
            .root()
            .and_then(|root| wallpaper::pick(root, seed))
            .and_then(|path| wallpaper_face(&path))
        else {
            return;
        };
        let id = compositor.create_texture(OUT_W, OUT_H, &rgba);
        self.session.app_mut().set_wallpaper(id);
    }

    /// Tells the shelf how tall the panel is, so on a 4:3 panel it can use the
    /// room under the 3:2 canvas.
    pub fn fit_panel(&mut self, panel: (u32, u32)) {
        if !slot_gfx::framed(panel) {
            return;
        }
        slot_ui::set_shelf_slack((OUT_W * panel.1 / panel.0).saturating_sub(OUT_H) as f32);
    }

    pub fn render(&mut self, compositor: &mut Compositor, window: (u32, u32)) {
        compositor.fit(window);
        self.compose(compositor);
        compositor.end_frame(window);
    }

    pub fn compose(&mut self, compositor: &mut Compositor) {
        compositor.set_frame_lift(self.session.app().frame_lift());
        compositor.set_frame_split(self.session.app().frame_split());
        compositor.begin_frame();
        if self.draw_rebuild(compositor) {
            return;
        }
        self.cart_faces
            .sync(self.session.app_mut(), compositor, false);
        sync_clock(self.session.app_mut(), compositor, &mut self.clocks);
        let want = self.session.app().wanted_game_name();
        if let Some(name) = want.filter(|n| self.game_named.as_ref() != Some(n)) {
            let tex = upload(compositor, &mut self.game_name_tex, game_name_face(&name));
            self.session.app_mut().set_game_name_face(name.clone(), tex);
            self.game_named = Some(name);
        }
        self.preload_box_pictures();
        let want = self.session.app().wanted_box_art().map(|p| p.to_path_buf());
        if let Some(path) = want.filter(|p| self.boxed.as_ref() != Some(p)) {
            match poll_box(&mut self.box_loading, &path) {
                Some(Some(f)) => {
                    let tex = upload_rgba(compositor, &mut self.box_tex, f.w, f.h, &f.rgba);
                    let app = self.session.app_mut();
                    app.set_box_face(path.clone(), tex, f.w, f.h);
                    self.boxed = Some(path);
                }
                Some(None) => self.session.app_mut().box_art_failed(),
                None => {}
            }
        }
        let want = self
            .session
            .app()
            .wanted_box_back()
            .map(|p| p.to_path_buf());
        if let Some(path) = want.filter(|p| self.backed.as_ref() != Some(p)) {
            match poll_box(&mut self.back_loading, &path) {
                Some(Some(f)) => {
                    let tex = upload_rgba(compositor, &mut self.box_back_tex, f.w, f.h, &f.rgba);
                    let app = self.session.app_mut();
                    app.set_box_back_face(path.clone(), tex, f.w, f.h);
                    self.backed = Some(path);
                }
                Some(None) => self.session.app_mut().box_back_failed(),
                None => {}
            }
        }
        sync_about(self.session.app_mut(), compositor, &mut self.about);
        sync_quick_clock(self.session.app_mut(), compositor, &mut self.quick_clock);
        self.draws.clear();
        self.session.app().draw(&mut self.draws);
        compositor.draw_list(&self.draws);
    }

    /// Once the shelf has rested on a cart long enough for its rom to be
    /// read ahead, start loading its box front and back too, so B shows
    /// them at once.
    fn preload_box_pictures(&mut self) {
        let Some(rom) = self.session.prefetching().map(|p| p.to_path_buf()) else {
            return;
        };
        if self.box_preloaded_for.as_ref() == Some(&rom) {
            return;
        }
        let Some((front, back)) = self.session.app().box_pictures_of(&rom) else {
            return;
        };
        self.box_preloaded_for = Some(rom);
        if let Some(front) = front.filter(|p| self.boxed.as_ref() != Some(p)) {
            poll_box(&mut self.box_loading, &front);
        }
        if let Some(back) = back.filter(|p| self.backed.as_ref() != Some(p)) {
            poll_box(&mut self.back_loading, &back);
        }
    }

    pub fn rebuilding_labels(&self) -> bool {
        self.rebuild.is_some()
    }

    fn check_labels(
        root: &std::path::Path,
        carts: Vec<slot_store::Cart>,
    ) -> Option<std::sync::mpsc::Receiver<Vec<slot_store::Cart>>> {
        let (tx, rx) = std::sync::mpsc::channel();
        let root = root.to_path_buf();
        std::thread::Builder::new()
            .name("slot-label-check".into())
            .spawn(move || {
                let _ = tx.send(label_cache::stale(&root, carts.iter()));
            })
            .ok()
            .map(|_| rx)
    }

    fn poll_label_check(&mut self) {
        let Some(rx) = &self.stale_check else {
            return;
        };
        let stale = match rx.try_recv() {
            Ok(stale) => stale,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Vec::new(),
        };
        self.stale_check = None;
        let Some(root) = self.session.app().root().map(|r| r.to_path_buf()) else {
            return;
        };
        if !stale.is_empty() {
            eprintln!("slot: label cache: {} new labels", stale.len());
            self.rebuild = Some(Rebuild::start(&root, stale));
        }
    }

    fn draw_rebuild(&mut self, compositor: &mut Compositor) -> bool {
        self.poll_label_check();
        self.poll_ui_faces(compositor);
        let Some(rebuild) = &self.rebuild else {
            return false;
        };
        if rebuild.finished() {
            self.rebuild = None;
            self.last = Instant::now();
            return false;
        }
        let faces = &mut self.rebuild_faces;
        let title = *faces.title.get_or_insert_with(|| {
            let f = rebuild_title_face("Caching New Labels");
            (compositor.create_texture(f.w, f.h, &f.rgba), f.w, f.h)
        });
        let done = rebuild.done();
        if faces.shown != Some(done) {
            let f = rebuild_count_face(&format!("{done} / {}", rebuild.total()));
            faces.count_size = (f.w, f.h);
            upload(compositor, &mut faces.count, f);
            faces.shown = Some(done);
        }
        let screen = RebuildScreen {
            title: Some(title),
            count: faces
                .count
                .map(|tex| (tex, faces.count_size.0, faces.count_size.1)),
            fraction: done as f32 / rebuild.total().max(1) as f32,
        };
        self.draws.clear();
        screen.draw(&mut self.draws);
        compositor.draw_list(&self.draws);
        true
    }

    pub fn advance(&mut self, input: &mut dyn InputSource) {
        let now = self.now();
        let events = input.poll(now);
        if self.rebuild.is_some() {
            return;
        }
        self.session.feed(events, now);
        let dt = self.last.elapsed().as_secs_f32();
        self.last = Instant::now();
        self.session.update(dt);
    }

    pub fn app(&self) -> &crate::app::App {
        self.session.app()
    }

    pub fn advance_at(&mut self, input: &mut dyn InputSource, now: Millis, dt: f32) {
        let events = input.poll(now);
        self.session.feed(events, now);
        self.session.update(dt);
    }

    fn now(&self) -> Millis {
        self.start.elapsed().as_millis() as Millis
    }

    pub fn powering_off(&self) -> bool {
        self.session.app().ready_to_power_off()
    }

    pub fn restarting(&self) -> bool {
        self.session.app().ready_to_restart()
    }

    pub fn exiting(&self) -> bool {
        self.session.app().exiting()
    }

    pub fn handed_off(&self) -> Option<&std::path::Path> {
        self.session.app().handed_off()
    }

    pub fn restart(&mut self) {
        self.session.app_mut().restart();
    }

    pub fn poweroff(&mut self) {
        self.session.app_mut().poweroff();
    }
}

struct UiFaces {
    shutdown: Vec<UndoFace>,
    power_menu: Vec<UndoFace>,
    quick_labels: Vec<UndoFace>,
    quick_values: Vec<[UndoFace; 2]>,
    quick_carets: [UndoFace; 2],
    quick_legend: [UndoFace; 3],
    help: Vec<UndoFace>,
    toasts: Vec<CartFace>,
}

impl UiFaces {
    fn build() -> Self {
        UiFaces {
            shutdown: PowerChoice::ALL
                .iter()
                .map(|c| {
                    menu_face(match c {
                        PowerChoice::Restart => "Restarting",
                        PowerChoice::PowerOff => "Powering Down",
                        PowerChoice::Exit => "Exiting",
                    })
                })
                .collect(),
            power_menu: PowerChoice::ALL
                .iter()
                .map(|c| menu_face(c.text()))
                .collect(),
            quick_labels: QuickRow::ALL.iter().map(|r| quick_label_face(*r)).collect(),
            quick_values: QuickValue::ALL
                .iter()
                .map(|v| [false, true].map(|lit| quick_value_face(v.text(), lit)))
                .collect(),
            quick_carets: [false, true].map(quick_caret_face),
            quick_legend: quick_legend_faces(),
            help: slot_ui::help_faces(),
            toasts: Toast::all().into_iter().map(toast_face).collect(),
        }
    }
}

fn sync_clock(app: &mut App, compositor: &mut Compositor, clocks: &mut Clocks) {
    let picked = app.picker().map(|p| p.text());
    if picked != clocks.picked {
        clocks.picked = picked;
        if let (Some(face), Some(hint)) = (app.picker().map(|p| p.face()), clocks.hint) {
            let line = upload(compositor, &mut clocks.line, face);
            app.set_clock_faces(line, hint);
        }
    }
    let shown = hhmm(app.wall_secs());
    if shown != clocks.shown {
        let face = word_face(&shown);
        clocks.shown = shown;
        let w = face.w;
        let id = upload(compositor, &mut clocks.shelf, face);
        app.set_shelf_clock_face(id, w);
    }
    let platform_shown = app.slot_text().unwrap_or_default();
    if platform_shown != clocks.platform {
        clocks.platform = platform_shown.clone();
        if platform_shown.is_empty() {
            app.clear_shelf_platform();
        } else {
            let face = word_face(&platform_shown);
            let w = face.w;
            let id = upload(compositor, &mut clocks.platform_tex, face);
            app.set_shelf_platform_face(id, w);
        }
    }
    let battery_shown = app
        .battery()
        .map(|b| format!("{}%", b.percent))
        .unwrap_or_default();
    if battery_shown != clocks.battery {
        clocks.battery = battery_shown.clone();
        if !battery_shown.is_empty() {
            let face = word_face(&battery_shown);
            let w = face.w;
            let id = upload(compositor, &mut clocks.battery_tex, face);
            app.set_battery_percent_face(id, w);
        }
    }
}

fn sync_quick_clock(app: &mut App, compositor: &mut Compositor, state: &mut QuickClock) {
    if app.quick_menu().is_none() {
        return;
    }
    let text = date_time_text(app.wall_secs());
    if text == state.shown {
        return;
    }
    let (dim, lit) = (
        quick_value_face(&text, false),
        quick_value_face(&text, true),
    );
    let (dim_size, lit_size) = ((dim.w, dim.h), (lit.w, lit.h));
    let dim = upload(compositor, &mut state.dim, dim);
    let lit = upload(compositor, &mut state.lit, lit);
    app.set_quick_clock_faces((dim, dim_size.0, dim_size.1), (lit, lit_size.0, lit_size.1));
    state.shown = text;
}

fn sync_about(app: &mut App, compositor: &mut Compositor, state: &mut AboutFace) {
    if !matches!(app.phase(), Phase::About) {
        return;
    }
    let battery = app.battery().map(|b| b.percent);
    if state.tex.is_some() && state.battery == battery {
        return;
    }
    state.battery = battery;
    let build = Build::current();
    let face = sticker_face(&StickerFields {
        battery,
        serial: &build.serial(),
        dirty_digit: build.dirty_digit(),
    });
    let id = upload(compositor, &mut state.tex, face);
    app.set_sticker_face(id);
}

fn upload(compositor: &mut Compositor, slot: &mut Option<TexId>, face: slot_ui::UndoFace) -> TexId {
    upload_rgba(compositor, slot, face.w, face.h, &face.rgba)
}

/// A box scan being read and shrunk off the drawing thread.
type BoxLoad = (
    std::path::PathBuf,
    std::sync::mpsc::Receiver<Option<slot_ui::CartFace>>,
);

/// The box scan at `path`, once it has loaded: `None` while it is still
/// loading, `Some(None)` if it would not load. Starts loading it, in the
/// background, the first time it is asked for.
fn poll_box(
    loading: &mut Option<BoxLoad>,
    path: &std::path::Path,
) -> Option<Option<slot_ui::CartFace>> {
    if loading.as_ref().map(|l| l.0.as_path()) != Some(path) {
        let (tx, rx) = std::sync::mpsc::channel();
        let from = path.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("slot-box-art".into())
            .spawn(move || {
                let _ = tx.send(slot_ui::box_face(&from));
            });
        if spawned.is_err() {
            *loading = None;
            return Some(slot_ui::box_face(path));
        }
        *loading = Some((path.to_path_buf(), rx));
    }
    let result = match loading.as_ref()?.1.try_recv() {
        Ok(face) => face,
        Err(std::sync::mpsc::TryRecvError::Empty) => return None,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
    };
    *loading = None;
    Some(result)
}

fn upload_rgba(
    compositor: &mut Compositor,
    slot: &mut Option<TexId>,
    w: u32,
    h: u32,
    rgba: &[u8],
) -> TexId {
    match *slot {
        Some(id) => {
            compositor.update_texture(id, w, h, rgba);
            id
        }
        None => {
            let id = compositor.create_texture(w, h, rgba);
            *slot = Some(id);
            id
        }
    }
}
