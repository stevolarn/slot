use std::time::Instant;

use shelf_gfx::{Compositor, Draw, TexId, OUT_H, OUT_W};
use shelf_input::{InputSource, Millis};
use shelf_power::{Platform, Power};
use shelf_ui::{
    cart_shadow, console_cart_shadow, date_time_text, game_name_face, gb_cart_shadow, hhmm,
    icon_face, menu_face, quick_caret_face, quick_heading_face, quick_label_face,
    quick_legend_faces, quick_value_face, rebuild_count_face, rebuild_title_face,
    set_clock_hint_face, sticker_face, toast_face, wallpaper_face, word_face, CartFace, Console,
    GbShell, Icon, PowerChoice, QuickMenuFaces, QuickRow, QuickValue, RebuildScreen, StickerFields,
    Toast, UndoFace, BOLT_PX, HUD_ICON_PX, HUD_INK,
};

use crate::app::{App, Phase};
use crate::build_info::Build;
use crate::cart_faces::CartFaces;
use crate::idle::{Idle, Look};
use crate::label_cache::{self, Rebuild};
use crate::session::Session;
use crate::wallpaper;

/// The longest time step one frame hands the app, in seconds.
const MAX_FRAME_GAP: f32 = 0.05;

pub struct Frontend {
    session: Session,
    start: Instant,
    last: Instant,
    draws: Vec<Draw>,
    /// The draw list and look of the frame on the screen now.
    shown: Vec<Draw>,
    shown_look: Option<Look>,
    idle: Idle,
    game_name_tex: Option<TexId>,
    game_named: Option<String>,
    details_line_tex: Option<TexId>,
    details_lined: Option<(std::path::PathBuf, Option<String>)>,
    back_details_tex: Option<TexId>,
    back_details_made: Option<crate::app::BackKey>,
    info_tex: Option<TexId>,
    cart_hints_tex: Option<TexId>,
    info_made: Option<crate::app::InfoKey>,
    box_tex: Option<TexId>,
    boxed: Option<std::path::PathBuf>,
    box_back_tex: Option<TexId>,
    backed: Option<std::path::PathBuf>,
    box_loading: Option<BoxLoad>,
    back_loading: Option<BoxLoad>,
    box_preloaded_for: Option<std::path::PathBuf>,
    /// The store's picture and price tags, while they are being made.
    store_rx: Option<std::sync::mpsc::Receiver<UndoFace>>,
    /// Shelf tag textures let go, to be drawn over for the next tags.
    stat_tex_free: Vec<TexId>,
    sign_tex: Option<TexId>,
    tab_tex: Option<TexId>,
    /// The section the sign and the tab are printed for.
    sectioned: Option<String>,
    cart_faces: CartFaces,
    rebuild: Option<Rebuild>,
    stale_check: Option<std::sync::mpsc::Receiver<Vec<shelf_store::Cart>>>,
    label_check: LabelCheck,
    composed: bool,
    ui_faces: Option<std::sync::mpsc::Receiver<UiFaces>>,
    rebuild_faces: RebuildFaces,
    clocks: Clocks,
    about: AboutFace,
    /// Two, so the receipt going keeps its face while the next one shows.
    receipt: [Option<TexId>; 2],
    receipt_slot: usize,
    artwork: ArtworkTex,
    quick_clock: QuickClock,
    wallpaper: Option<std::sync::mpsc::Receiver<Option<Vec<u8>>>>,
}

/// Whether the check for labels not cached yet has started. It waits for
/// the first frame, so it does not compete with the boot for the card.
enum LabelCheck {
    Waiting,
    Started,
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

/// The Get Artwork screen's textures, kept to be written over as its
/// words change.
#[derive(Default)]
struct ArtworkTex {
    title: Option<TexId>,
    lines: Vec<Option<TexId>>,
    hints: Vec<Option<TexId>>,
}

#[derive(Default)]
struct AboutFace {
    tex: Option<TexId>,
    battery: Option<u8>,
    card: Option<TexId>,
}

#[derive(Default)]
struct Clocks {
    line: Option<TexId>,
    hint: Option<TexId>,
    shelf: Option<TexId>,
    picked: Option<String>,
    shown: String,
    date: Option<TexId>,
    store_date: Option<TexId>,
    dated: String,
    battery: String,
    battery_tex: Option<TexId>,
    platform: String,
    platform_tex: Option<TexId>,
    ticket_tex: Option<TexId>,
}

impl Frontend {
    pub fn boot(platform: Box<dyn Platform>) -> Self {
        Self::boot_with(platform, crate::audio::opened_sink())
    }

    pub fn boot_with(platform: Box<dyn Platform>, sink: Box<dyn crate::audio::AudioSink>) -> Self {
        let app = App::boot(platform.root());
        Self::boot_prepared(platform, sink, app)
    }

    /// Boot around an app that was built ahead, on another thread.
    pub fn boot_prepared(
        platform: Box<dyn Platform>,
        sink: Box<dyn crate::audio::AudioSink>,
        app: App,
    ) -> Self {
        let now = Instant::now();
        let cart_faces = CartFaces::spawn(platform.root().to_path_buf());
        let mut session = Session::with_app(app, sink);
        crate::boot_time::mark("session");
        session.app_mut().set_power(Power::new(platform));
        Frontend {
            session,
            start: now,
            last: now,
            draws: Vec::new(),
            shown: Vec::new(),
            shown_look: None,
            idle: Idle::default(),
            game_name_tex: None,
            game_named: None,
            details_line_tex: None,
            details_lined: None,
            back_details_tex: None,
            back_details_made: None,
            info_tex: None,
            cart_hints_tex: None,
            info_made: None,
            box_tex: None,
            boxed: None,
            box_back_tex: None,
            backed: None,
            box_loading: None,
            back_loading: None,
            box_preloaded_for: None,
            store_rx: None,
            stat_tex_free: Vec::new(),
            sign_tex: None,
            tab_tex: None,
            sectioned: None,
            cart_faces,
            rebuild: None,
            stale_check: None,
            label_check: LabelCheck::Waiting,
            composed: false,
            ui_faces: None,
            rebuild_faces: RebuildFaces::default(),
            clocks: Clocks::default(),
            about: AboutFace::default(),
            receipt: [None, None],
            receipt_slot: 0,
            artwork: ArtworkTex::default(),
            quick_clock: QuickClock::default(),
            wallpaper: None,
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
            .name("shelf-ui-faces".into())
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
        for console in Console::ALL {
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
        let labels = faces.quick_labels.into_iter().map(&mut up).collect();
        let values = faces
            .quick_values
            .into_iter()
            .map(|v| v.map(&mut up))
            .collect();
        let carets = faces.quick_carets.map(&mut up);
        let heading = up(faces.quick_heading);
        let legend = faces.quick_legend.map(|f| {
            let (tex, w, _) = up(f);
            (tex, w)
        });
        let saved = up(faces.saved);
        let note = up(faces.note);
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
        app.set_help_faces(help);
        app.set_saved_face(saved);
        app.set_note_face(note);
        app.set_quick_menu_faces(QuickMenuFaces {
            labels,
            values,
            carets,
            heading: Some(heading),
            legend,
        });
        app.set_toast_faces(toasts);
    }

    /// Take the wallpaper from a thread that is already picking and
    /// decoding it, rather than doing that when the first faces go up.
    pub fn set_wallpaper_loader(&mut self, rx: std::sync::mpsc::Receiver<Option<Vec<u8>>>) {
        self.wallpaper = Some(rx);
    }

    fn upload_wallpaper(&mut self, compositor: &mut Compositor) {
        let rgba = match self.wallpaper.take() {
            Some(rx) => rx.recv().ok().flatten(),
            None => {
                let app = self.session.app();
                let seed = app.wall_secs().unsigned_abs();
                app.root()
                    .and_then(|root| wallpaper::pick(root, seed))
                    .and_then(|path| wallpaper_face(&path))
            }
        };
        let Some(rgba) = rgba else {
            return;
        };
        let id = compositor.create_texture(OUT_W, OUT_H, &rgba);
        self.session.app_mut().set_wallpaper(id);
    }

    /// Tells the shelf how tall the panel is, so on a 4:3 panel it can use the
    /// room under the 3:2 canvas.
    pub fn fit_panel(&mut self, panel: (u32, u32)) {
        if !shelf_gfx::framed(panel) {
            return;
        }
        shelf_ui::set_shelf_slack((OUT_W * panel.1 / panel.0).saturating_sub(OUT_H) as f32);
    }

    /// Draw the frame if it would differ from the one on the screen, and
    /// say whether it was drawn: when it was, the caller swaps; when it was
    /// not, the screen already shows it and there is nothing to swap.
    pub fn render(&mut self, compositor: &mut Compositor, window: (u32, u32)) -> bool {
        let writes = compositor.texture_writes();
        self.prepare(compositor);
        let look = Look {
            lift: self.session.app().frame_lift(),
            split: self.session.app().frame_split(),
            window,
        };
        let same = compositor.texture_writes() == writes
            && self.shown_look == Some(look)
            && shelf_gfx::same_frame(&self.draws, &self.shown);
        if same {
            self.idle.skipped();
            return false;
        }
        compositor.fit(window);
        self.paint(compositor);
        compositor.end_frame(window);
        std::mem::swap(&mut self.draws, &mut self.shown);
        self.shown_look = Some(look);
        self.idle.drawn();
        true
    }

    /// Whether the screen has stayed the same long enough, and nothing is
    /// held or loading, that the loop may wait for input instead of
    /// stepping every frame.
    pub fn at_rest(&self) -> bool {
        self.idle.rested() && !self.busy()
    }

    /// How long the loop may wait for input once at rest.
    pub fn rest_tick(&self) -> std::time::Duration {
        self.idle.tick()
    }

    fn busy(&self) -> bool {
        self.session.busy()
            || self.cart_faces.busy()
            || self.rebuild.is_some()
            || self.stale_check.is_some()
            || !self.composed
            || self.box_loading.is_some()
            || self.back_loading.is_some()
            || self.store_rx.is_some()
            || self.ui_faces.is_some()
            || self.wallpaper.is_some()
            || self
                .session
                .prefetching()
                .is_some_and(|rom| self.box_preloaded_for.as_deref() != Some(rom))
    }

    /// Draw the frame whether or not it differs from the last.
    pub fn compose(&mut self, compositor: &mut Compositor) {
        self.prepare(compositor);
        self.paint(compositor);
    }

    fn paint(&mut self, compositor: &mut Compositor) {
        compositor.set_frame_lift(self.session.app().frame_lift());
        compositor.set_frame_split(self.session.app().frame_split());
        compositor.begin_frame();
        compositor.draw_list(&self.draws);
    }

    /// Bring the textures up to date and work out what the frame is to
    /// show, in `self.draws`.
    fn prepare(&mut self, compositor: &mut Compositor) {
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
        self.sync_store(compositor);
        self.sync_sections(compositor);
        self.sync_stat_tags(compositor);
        self.preload_box_pictures();
        self.sync_details(compositor);
        self.sync_info(compositor);
        if !self.session.app().cart_hints_made() {
            let f = shelf_ui::cart_hints_face();
            let tex = upload_rgba(compositor, &mut self.cart_hints_tex, f.w, f.h, &f.rgba);
            self.session.app_mut().set_cart_hints_face(tex, f.w, f.h);
        }
        let want = self.session.app().wanted_box_art().map(|p| p.to_path_buf());
        if let Some(path) = want.filter(|p| self.boxed.as_ref() != Some(p)) {
            match poll_box(&mut self.box_loading, &path, shelf_ui::box_face) {
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
            match poll_box(&mut self.back_loading, &path, shelf_ui::box_back_scan) {
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
        sync_receipt(
            self.session.app_mut(),
            compositor,
            &mut self.receipt,
            &mut self.receipt_slot,
        );
        sync_artwork(self.session.app_mut(), compositor, &mut self.artwork);
        sync_quick_clock(self.session.app_mut(), compositor, &mut self.quick_clock);
        self.draws.clear();
        self.session.app().draw(&mut self.draws);
        self.composed = true;
    }

    /// Make the store's picture and price tags, off the drawing thread, the
    /// first time the store is shown.
    fn sync_store(&mut self, compositor: &mut Compositor) {
        if self.store_rx.is_none() && self.session.app().wants_store_art() {
            let (tx, rx) = std::sync::mpsc::channel();
            let spawned = std::thread::Builder::new()
                .name("shelf-store-art".into())
                .spawn(move || {
                    let _ = tx.send(shelf_ui::store_backdrop_face());
                });
            match spawned {
                Ok(_) => self.store_rx = Some(rx),
                Err(_) => {
                    let made = shelf_ui::store_backdrop_face();
                    self.place_store(compositor, made);
                }
            }
        }
        let Some(rx) = &self.store_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(made) => {
                self.store_rx = None;
                self.place_store(compositor, made);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.store_rx = None,
        }
    }

    fn place_store(&mut self, compositor: &mut Compositor, back: UndoFace) {
        let back = compositor.create_texture(back.w, back.h, &back.rgba);
        self.session.app_mut().set_store_art(back);
    }

    /// Make the store's shelf tags for the games on screen, a few a frame:
    /// each is a line of small print.
    fn sync_stat_tags(&mut self, compositor: &mut Compositor) {
        let shed = self.session.app_mut().shed_stat_tags();
        self.stat_tex_free.extend(shed);
        for tag in self.session.app().wanted_stat_tags().into_iter().take(4) {
            let face = shelf_ui::stat_tag_face(&tag);
            let (w, h) = (face.w, face.h);
            let mut slot = self.stat_tex_free.pop();
            let tex = upload(compositor, &mut slot, face);
            self.session.app_mut().set_stat_tag_face(tag, (tex, w, h));
        }
    }

    /// Print the section the middle game is in, for the aisle sign and the
    /// cart shelf's tab, whenever it changes.
    fn sync_sections(&mut self, compositor: &mut Compositor) {
        let Some(label) = self.session.app().wanted_section() else {
            return;
        };
        if self.sectioned.as_ref() == Some(&label) {
            return;
        }
        let (sign, tab) = (shelf_ui::sign_face(&label), shelf_ui::tab_face(&label));
        let (sign_size, tab_size) = ((sign.w, sign.h), (tab.w, tab.h));
        let sign = upload(compositor, &mut self.sign_tex, sign);
        let tab = upload(compositor, &mut self.tab_tex, tab);
        self.session
            .app_mut()
            .set_section_faces(crate::app::SectionFaces {
                label: label.clone(),
                sign: (sign, sign_size.0, sign_size.1),
                tab: (tab, tab_size.0, tab_size.1),
            });
        self.sectioned = Some(label);
    }

    /// Make the faces the game's details are shown on, each once: the
    /// line under its name, and its box back.
    fn sync_details(&mut self, compositor: &mut Compositor) {
        let want = self.session.app().wanted_details_line();
        let extra = self.session.app().details_line_plays();
        if let Some((rom, details)) =
            want.filter(|w| self.details_lined != Some((w.0.clone(), extra.clone())))
        {
            if let Some(face) = shelf_ui::details_line_face_with(&details, extra.as_deref()) {
                let tex = upload(compositor, &mut self.details_line_tex, face);
                self.session
                    .app_mut()
                    .set_details_line_face(rom.clone(), tex);
            }
            self.details_lined = Some((rom, extra));
        }
        let Some(key) = self.session.app().back_details_key() else {
            return;
        };
        if self.back_details_made.as_ref() == Some(&key) {
            return;
        }
        let Some(details) = self.session.app().back_details().cloned() else {
            return;
        };
        let face = match key.strip {
            true => shelf_ui::back_strip_face(&details, key.w, key.h, key.rental.as_ref()),
            false => shelf_ui::back_face(
                &details,
                &key.stem,
                key.w,
                key.h,
                key.panel,
                key.rental.as_ref(),
            ),
        };
        let tex = upload_rgba(
            compositor,
            &mut self.back_details_tex,
            face.w,
            face.h,
            &face.rgba,
        );
        self.session
            .app_mut()
            .set_back_details_face(key.clone(), tex, face.w, face.h);
        self.back_details_made = Some(key);
    }

    /// Make the info card for the game whose box is on its back, once, so
    /// that it is ready when B brings it up.
    fn sync_info(&mut self, compositor: &mut Compositor) {
        let Some(key) = self.session.app().info_key() else {
            return;
        };
        if self.info_made.as_ref() == Some(&key) {
            return;
        }
        let Some((details, platform, stem, paths)) = self.session.app().info_source() else {
            return;
        };
        let shots: Vec<_> = paths
            .iter()
            .filter_map(|p| shelf_ui::info_shot(p))
            .collect();
        let face = shelf_ui::info_face(&details, &stem, platform, &shots, key.rental.as_ref());
        let tex = upload_rgba(compositor, &mut self.info_tex, face.w, face.h, &face.rgba);
        self.session.app_mut().set_info_face(key.clone(), tex);
        self.info_made = Some(key);
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
        self.box_preloaded_for = Some(rom.clone());
        self.session.app_mut().load_details_of(&rom);
        if let Some(front) = front.filter(|p| self.boxed.as_ref() != Some(p)) {
            poll_box(&mut self.box_loading, &front, shelf_ui::box_face);
        }
        if let Some(back) = back.filter(|p| self.backed.as_ref() != Some(p)) {
            poll_box(&mut self.back_loading, &back, shelf_ui::box_back_scan);
        }
    }

    pub fn rebuilding_labels(&self) -> bool {
        self.rebuild.is_some()
    }

    fn check_labels(
        root: &std::path::Path,
        carts: Vec<shelf_store::Cart>,
    ) -> Option<std::sync::mpsc::Receiver<Vec<shelf_store::Cart>>> {
        let (tx, rx) = std::sync::mpsc::channel();
        let root = root.to_path_buf();
        std::thread::Builder::new()
            .name("shelf-label-check".into())
            .spawn(move || {
                let _ = tx.send(label_cache::missing_copies(&root, carts.iter()));
            })
            .ok()
            .map(|_| rx)
    }

    fn poll_label_check(&mut self) {
        if matches!(self.label_check, LabelCheck::Waiting) {
            if !self.composed {
                return;
            }
            self.label_check = LabelCheck::Started;
            let carts = self.session.app().carts().cloned().collect();
            if let Some(root) = self.session.app().root().map(|r| r.to_path_buf()) {
                self.stale_check = Self::check_labels(&root, carts);
            }
        }
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
            eprintln!("shelf: label cache: {} games with new pictures", stale.len());
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
            let f = rebuild_title_face("Caching New Artwork");
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
        true
    }

    pub fn advance(&mut self, input: &mut dyn InputSource) {
        let now = self.now();
        let events = input.poll(now);
        if self.rebuild.is_some() {
            return;
        }
        self.session.feed(events, now);
        // After the loop has slept on a still screen the gap can be long;
        // nothing was moving meanwhile, so carry on from a normal frame.
        let dt = self.last.elapsed().as_secs_f32().min(MAX_FRAME_GAP);
        self.last = Instant::now();
        self.session.update(dt);
    }

    /// Whether a tap on power asked to sleep, clearing the ask.
    pub fn take_sleep(&mut self) -> bool {
        self.session.app_mut().take_sleep()
    }

    /// Sleep: screen off, suspend, and on waking the screen back on, with
    /// what was pressed meanwhile thrown away and power presses ignored for
    /// a second, since the power chip reports a spurious one on resume.
    pub fn sleep(&mut self, input: &mut dyn InputSource) {
        let began = Instant::now();
        let outcome = self.session.app_mut().suspend();
        if outcome == shelf_power::Suspend::Unavailable {
            // No suspend script: keep the screen off until the next press.
            sleep_until_power_press(input, || self.start.elapsed().as_millis() as Millis);
        }
        self.session.app_mut().wake();
        let now = self.now();
        let _ = input.poll(now);
        self.session
            .ignore_power_until(now + shelf_input::POWER_SPURIOUS_MS);
        self.last = Instant::now();
        // The screen may have lost its picture: draw it again.
        self.shown_look = None;
        eprintln!("shelf: slept {:.1} s", began.elapsed().as_secs_f64());
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

    pub fn resume_saves(&self) -> bool {
        self.session.app().resume_saves()
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

/// Wait, with the screen off, for the power button to go down. Whatever
/// else is pressed is thrown away.
pub fn sleep_until_power_press(input: &mut dyn InputSource, mut now: impl FnMut() -> Millis) {
    use shelf_input::{Btn, RawEvent};
    loop {
        let events = input.poll(now());
        if events.contains(&RawEvent::Down(Btn::Power)) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

struct UiFaces {
    shutdown: Vec<UndoFace>,
    saved: UndoFace,
    note: UndoFace,
    quick_heading: UndoFace,
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
            saved: word_face("SAVED"),
            note: shelf_ui::continue_note_face(),
            quick_heading: quick_heading_face(),
            quick_labels: QuickRow::ALL
                .iter()
                .map(|r| quick_label_face(*r))
                .chain([shelf_ui::quick_unhide_face()])
                .collect(),
            quick_values: QuickValue::ALL
                .iter()
                .map(|v| [false, true].map(|lit| quick_value_face(v.text(), lit)))
                .collect(),
            quick_carets: [false, true].map(quick_caret_face),
            quick_legend: quick_legend_faces(),
            help: shelf_ui::help_faces(),
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
    let dated = shelf_ui::day_month(app.wall_secs());
    if dated != clocks.dated {
        let face = shelf_ui::date_face(&dated);
        let w = face.w;
        let id = upload(compositor, &mut clocks.date, face);
        app.set_shelf_date_face(id, w);
        let face = shelf_ui::vcr_date_face(&dated);
        let w = face.w;
        let id = upload(compositor, &mut clocks.store_date, face);
        app.set_store_date_face(id, w);
        clocks.dated = dated;
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
            let ticket = shelf_ui::ticket_face(&platform_shown);
            let (tw, th) = (ticket.w, ticket.h);
            if tw > 0 {
                let id = upload(compositor, &mut clocks.ticket_tex, ticket);
                app.set_store_ticket_face(id, tw, th);
            }
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

fn sync_artwork(app: &mut App, compositor: &mut Compositor, state: &mut ArtworkTex) {
    let Some(text) = app.screen_text() else {
        return;
    };
    if app.artwork_faces_text() == Some(&text) {
        return;
    }
    let face = shelf_ui::screen_title_face(text.title);
    let (w, h) = (face.w, face.h);
    let title = (upload(compositor, &mut state.title, face), w, h);
    state
        .lines
        .resize(state.lines.len().max(text.lines.len()), None);
    let lines = text
        .lines
        .iter()
        .zip(state.lines.iter_mut())
        .map(|(line, slot)| {
            let face = shelf_ui::artwork_line_face(line);
            let (w, h) = (face.w, face.h);
            (upload(compositor, slot, face), w, h)
        })
        .collect();
    state
        .hints
        .resize(state.hints.len().max(text.hints.len()), None);
    let hints = text
        .hints
        .iter()
        .zip(state.hints.iter_mut())
        .map(|((key, label), slot)| {
            let face = shelf_ui::artwork_hint_face(key, label);
            let w = face.w;
            (upload(compositor, slot, face), w)
        })
        .collect();
    app.set_artwork_faces(shelf_ui::ArtworkFaces {
        text,
        title,
        lines,
        hints,
    });
}

fn sync_receipt(
    app: &mut App,
    compositor: &mut Compositor,
    slots: &mut [Option<TexId>; 2],
    at: &mut usize,
) {
    let Some(key) = app.receipt_wanted() else {
        return;
    };
    if app.receipt_face_key() == Some(&key) {
        return;
    }
    let face = shelf_ui::receipt_face(key.0.as_ref(), key.1);
    *at = 1 - *at;
    let id = upload(compositor, &mut slots[*at], face);
    app.set_receipt_face(key, id);
}

fn sync_about(app: &mut App, compositor: &mut Compositor, state: &mut AboutFace) {
    if !matches!(app.phase(), Phase::About) {
        return;
    }
    if let Some(card) = app.member_card().filter(|c| app.member_face_card() != Some(c)) {
        let face = shelf_ui::member_card_face(&shelf_ui::MemberCard {
            serial: Build::current().serial(),
            ..card.clone()
        });
        let id = upload(compositor, &mut state.card, face);
        app.set_member_face(card, id);
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

fn upload(
    compositor: &mut Compositor,
    slot: &mut Option<TexId>,
    face: shelf_ui::UndoFace,
) -> TexId {
    upload_rgba(compositor, slot, face.w, face.h, &face.rgba)
}

/// A box scan being read and shrunk off the drawing thread.
type BoxLoad = (
    std::path::PathBuf,
    std::sync::mpsc::Receiver<Option<shelf_ui::CartFace>>,
);

/// The box scan at `path`, once it has loaded: `None` while it is still
/// loading, `Some(None)` if it would not load. Starts loading it, in the
/// background, the first time it is asked for.
fn poll_box(
    loading: &mut Option<BoxLoad>,
    path: &std::path::Path,
    load: fn(&std::path::Path) -> Option<shelf_ui::CartFace>,
) -> Option<Option<shelf_ui::CartFace>> {
    if loading.as_ref().map(|l| l.0.as_path()) != Some(path) {
        let (tx, rx) = std::sync::mpsc::channel();
        let from = path.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("shelf-box-art".into())
            .spawn(move || {
                let _ = tx.send(load(&from));
            });
        if spawned.is_err() {
            *loading = None;
            return Some(load(path));
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
