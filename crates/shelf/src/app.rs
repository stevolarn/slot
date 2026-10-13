use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use shelf_gfx::{OUT_H, OUT_W};
use shelf_input::{Action, Btn, MUTE_CHORD_MS};
use shelf_power::{Battery, Charge, Power};
use shelf_store::{
    details_for, dust_amount, read_played, read_rentals, read_shelf_state, rent, scan_in,
    sections_worded, sorted_order, write_played, write_rentals, write_shelf_state, Cart, Details,
    DisplayMode, Platform, Played, Rental, Rentals, ShelfState, SortKey, Theme, BRIGHTNESS_MAX,
    VOLUME_MAX,
};
use shelf_ui::{
    draw_backdrop, draw_footer, draw_slot_name, draw_sticker, ease, ClockPicker, Draw, GbShell,
    Hud, HudKind, Icon, Millis, PowerChoice, QuickMenu, QuickMenuFaces, QuickRow, QuickValue,
    Refusal, Shelf, SlotChrome, TexId, Toast,
};

use crate::audio::Sfx;

/// How long a cart takes to go into the slot, from the first move to the
/// last, before the game is handed to NextUI.
pub const INSERT_S: f32 = 0.73;
const INSERT_HOLD_S: f32 = 0.28;

/// How far through the insert the cart is seated, as a time.
pub const SEATED_AT: f32 = INSERT_S - INSERT_HOLD_S;

const VOLUME_STEP: u8 = 5;

const CARD_CHECK_MS: Millis = 2_000;

const BATTERY_CRITICAL: u8 = 5;

const BATTERY_POLL_MS: Millis = 10_000;

const CHARGE_POLL_MS: Millis = 1_000;

const SHUTDOWN_SHOW_MS: Millis = 250;

const SLOT_NAME_IN_MS: Millis = 200;
const SLOT_NAME_HOLD_MS: Millis = 1200;
const SLOT_NAME_OUT_MS: Millis = 800;
const SLOT_NAME_ALPHA: f32 = 0.4;

const CLOCK_FLOOR: i64 = 1_577_836_800;

#[derive(Debug)]
pub enum Phase {
    SetClock {
        picker: ClockPicker,
        seed: i64,
        from_menu: bool,
    },
    Shelf,
    QuickMenu {
        row: QuickRow,
    },
    /// A cart sliding into the slot; once it is seated its game is handed
    /// to NextUI.
    Inserting {
        cart: String,
        t: f32,
    },
    About,
    /// The receipts on the spike, `at` from the newest.
    Receipts {
        at: usize,
    },
    /// Get Artwork: box art, cart photos and details from LaunchBox.
    Artwork,
    /// Search: a name spelt a letter at a time, as on an arcade's high
    /// score table, and the games it matches.
    Search {
        query: String,
        /// The letter being picked, from `SEARCH_LETTERS`.
        letter: usize,
        /// Which match A goes to.
        pick: usize,
    },
}

/// The receipt out of the till after a game: when it started printing,
/// once the shelf is showing, and when it was torn off.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Slip {
    pub at: Option<Millis>,
    pub torn: Option<Millis>,
}

/// What a receipt face shows: a receipt, or the empty spike in a theme.
pub type ReceiptKey = (Option<shelf_ui::ReceiptSlip>, bool);

/// What Up and Down step through on the search screen; a space first,
/// shown as `_`, for a letter not picked yet.
const SEARCH_LETTERS: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
/// How many matches the search screen lists at once.
const SEARCH_SHOWN: usize = 5;

/// How many of the store's shelf tags are kept made before the ones out of
/// sight are let go.
const STAT_TAGS_KEPT: usize = 40;

/// How far the game's name sits below the top of the screen.
const NAME_TOP: f32 = 14.0;

/// The highest the box shelf's boxes reach: a little below the game's
/// name and the line of details under it.
fn boxes_ceiling() -> f32 {
    NAME_TOP - shelf_ui::band()
        + shelf_ui::DETAILS_DROP
        + shelf_ui::DETAILS_H as f32
        + shelf_ui::BOXES_TITLE_GAP
}

pub struct App {
    phase: Phase,
    shelves: Vec<(ShelfKey, Shelf)>,
    /// Every game the scan found, hidden or not, to build shelves from.
    all_carts: Vec<Cart>,
    /// The games hidden from the shelves, from `Config/hidden.txt`.
    hidden: Vec<PathBuf>,
    /// The game going in plays from its start, not its last save state.
    fresh: bool,
    /// About shows the small print, the card turned over.
    about_back: bool,
    member_face: Option<(shelf_ui::MemberCard, TexId)>,
    /// Every game handed back, newest first, from `Config/receipts.txt`.
    receipts: Vec<shelf_store::Receipt>,
    /// The receipt printing after a game, while it is out.
    slip: Option<Slip>,
    receipt_face: Option<(ReceiptKey, TexId)>,
    shelf_at: usize,
    play_held: Option<Millis>,
    refusal: Option<Refusal>,
    blow: Option<shelf_ui::Blow>,
    /// The game being blown on; blowing keeps it clean once it is done.
    blow_rom: Option<PathBuf>,
    /// When each game was last played or blown on.
    played: Played,
    /// How often and when last each game was played, for the rental sticker.
    rentals: Rentals,
    /// Plays NextUI logged itself, from before Shelf counted them.
    nextui_plays: Rentals,
    /// How long each game has been played from Shelf, and per NextUI's log.
    play_times: shelf_store::PlayTimes,
    nextui_times: shelf_store::PlayTimes,
    /// Where NextUI keeps its save-state slots, and the Roms folder the
    /// systems' tags are read from, to tell which games can be continued.
    resume_lookup: Option<(PathBuf, PathBuf)>,
    /// Whether each game looked at has a save-state slot.
    saved: HashMap<PathBuf, bool>,
    saved_face: Option<(TexId, u32, u32)>,
    /// The CONTINUE? note tucked in the box of a game with a save.
    note_face: Option<(TexId, u32, u32)>,
    /// The store's shelf tags made so far, by what they say.
    stat_tags: HashMap<shelf_ui::StatTag, (TexId, u32, u32)>,
    help: bool,
    spin: Option<shelf_ui::Spin>,
    box_view: Option<shelf_ui::BoxView>,
    /// On the box shelf, the middle box opened to take its cart out.
    peek: Option<shelf_ui::BoxView>,
    /// A step along the shelf to take once the peeked box is shut.
    peek_step: Option<i32>,
    /// The game the peeked box is, so a shelf changed under it ends it.
    peek_rom: Option<PathBuf>,
    /// The box picture wanted for box view, and the face made from it.
    box_art: Option<PathBuf>,
    box_face: Option<(PathBuf, TexId, u32, u32)>,
    /// A scan of the box's back, when there is one, and its face.
    box_back: Option<PathBuf>,
    box_back_face: Option<(PathBuf, TexId, u32, u32)>,
    help_faces: Vec<(TexId, u32)>,
    /// The strip of button hints under a cart taken out, and its size.
    cart_hints: Option<(TexId, u32, u32)>,
    shutdown_faces: Vec<(TexId, u32, u32)>,
    /// What the menu was raised over, when that was not the shelf: it
    /// comes back when the menu closes.
    parked: Option<Phase>,
    restarting: bool,
    act_at: Millis,
    /// Get Artwork's run, once started, and where it fetches from.
    artwork: Option<shelf_fetch::Job>,
    artwork_net: Option<Arc<dyn shelf_fetch::Net>>,
    /// How many pictures the games lack, about how much room they take,
    /// and how much the card has free, as Get Artwork opened.
    artwork_need: (usize, usize, u64, Option<u64>),
    /// Each console's games lacking a front, a back and a cart photo.
    artwork_gaps: Vec<(Platform, [usize; 3])>,
    artwork_faces: Option<shelf_ui::ArtworkFaces>,
    root: Option<PathBuf>,
    state: ShelfState,
    vol_before: Vec<(u8, bool, Millis)>,
    sfx: Option<Sfx>,
    clock_faces: Option<(TexId, TexId)>,
    quick_menu_faces: Option<QuickMenuFaces>,
    quick_clock_faces: Option<[(TexId, u32, u32); 2]>,
    sticker_face: Option<TexId>,
    wallpaper: Option<TexId>,
    battery_percent: shelf_ui::Printed,
    bolt: Option<TexId>,
    shelf_platform: shelf_ui::Printed,
    /// The console's name on the store's ticket, with its size.
    store_ticket: Option<(TexId, u32, u32)>,
    game_name: Option<(String, TexId)>,
    /// What is known of each game the shelf has rested on, read once.
    details: HashMap<PathBuf, Option<Details>>,
    /// The line of details printed under the name, and the game it is for.
    details_line: Option<(PathBuf, TexId)>,
    /// The details printed on the box back, and what they were made for.
    back_details_face: Option<(BackKey, TexId, u32, u32)>,
    /// The info card in front of a box's back, coming up or going away.
    info: Option<InfoCard>,
    /// The info card made for a game, and what it was made for.
    info_face: Option<(InfoKey, TexId)>,
    name_pending: bool,
    /// What the slot shows for a moment after a jump: the letter, or the
    /// section jumped to.
    slot_letter: Option<String>,
    shelf_named: Option<Millis>,
    shelf_clock: shelf_ui::Printed,
    hud: Hud,
    clock: f64,
    power: Option<Power>,
    card_failing: Arc<AtomicBool>,
    card_read_only: bool,
    card_check_at: Millis,
    battery_at: Millis,
    charge_at: Millis,
    battery: Option<Battery>,
    powering_off: bool,
    exiting: bool,
    state_writer: crate::state_writer::StateWriter,
    handed: Option<PathBuf>,
    sleep_wanted: bool,
    /// How long with no button pressed before the Brick goes to sleep by
    /// itself: NextUI's screen timeout. None never does.
    screen_timeout: Option<Millis>,
    /// How long since a button was last pressed, counted in frames' time,
    /// so the time asleep is never counted.
    idle_ms: f64,
    /// Where the shelf was when faces were last shed, and nothing has been
    /// given a face since: shedding again would find nothing new.
    shed_at: Option<(usize, usize)>,
    /// Whether each shelf's details have all been read, for sorting by them.
    details_loaded: Vec<bool>,
    /// Whether each shelf has been put in an order other than by name.
    shelf_sorted: Vec<bool>,
    /// The details being read for every shelf, on a thread of their own.
    details_rx: Option<std::sync::mpsc::Receiver<ShelfDetails>>,
    /// Each game's rating and genres, read in the background for the
    /// Staff Picks without touching the shelves' order or the details shown.
    pick_facts: HashMap<PathBuf, (Option<f32>, Vec<String>)>,
    picks_rx: Option<std::sync::mpsc::Receiver<PickFacts>>,
    /// The section each cart on each shelf is in, in the shelf's order.
    section_labels: Vec<Vec<String>>,
    /// Which section the sign was last showing, and since when.
    sign_section: Option<String>,
    sign_at: Millis,
    /// The faces the section is printed on, and the section they are of.
    section_faces: Option<SectionFaces>,
    /// The store's picture and price tags.
    store_art: Option<TexId>,
    /// Counts the times the shelves changed under the faces being made, so
    /// the ones in flight are known to be for something else.
    face_epoch: u32,
}

/// What was read of each game's details on one shelf: its place, and the
/// details of each rom.
type ShelfDetails = (usize, Vec<(PathBuf, Option<Details>)>);

/// The aisle sign's lettering and the cart shelf's small tab, printed for
/// a section: each with its size.
#[derive(Clone, Debug)]
pub struct SectionFaces {
    pub label: String,
    pub sign: (TexId, u32, u32),
    pub tab: (TexId, u32, u32),
}

/// Which picture of a game a face is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FaceKind {
    /// The cart in its slot.
    Cart,
    /// The box front on the store shelf.
    Box,
    /// The larger box front on the box shelf.
    Tall,
}

const FACE_AHEAD: i32 = 8;
const FACE_KEEP: i32 = 12;
/// The same for the box shelf, whose pictures are several times larger and
/// of which only one shows whole.
/// The look-ahead must reach past the three each side the row can show,
/// or scrolling meets boxes still loading.
const TALL_AHEAD: i32 = 6;
const TALL_KEEP: i32 = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FaceWant {
    pub shelf: usize,
    pub index: usize,
    pub on_screen: bool,
    pub kind: FaceKind,
}

/// What a shelf holds: one console's games, the games played last, or
/// the games picked as favourites.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ShelfKey {
    Platform(Platform),
    /// Games Shelf suggests: ones left part way, and good ones never played.
    Picks,
    Recent,
    Favourites,
}

impl ShelfKey {
    pub fn name(self) -> &'static str {
        self.name_in(false)
    }

    /// The shelf's name, in plain words for the original theme.
    pub fn name_in(self, plain: bool) -> &'static str {
        match (self, plain) {
            (ShelfKey::Platform(p), _) => p.name(),
            (ShelfKey::Picks, false) => "Staff Picks",
            (ShelfKey::Picks, true) => "Picked for You",
            (ShelfKey::Recent, false) => "Recently Rented",
            (ShelfKey::Favourites, false) => "Favourite Rentals",
            (ShelfKey::Recent, true) => "Recently Played",
            (ShelfKey::Favourites, true) => "Favourites",
        }
    }

    /// Whether the shelf holds copies of carts from the console shelves,
    /// kept in its own order.
    pub fn is_list(self) -> bool {
        !matches!(self, ShelfKey::Platform(_))
    }
}

/// Each game's rating and genres, as the picks thread reads them.
type PickFacts = Vec<(PathBuf, Option<f32>, Vec<String>)>;

/// How many games the Staff Picks shelf holds.
pub const PICKS_MAX: usize = 10;

/// How many games the recently played shelf keeps.
pub const RECENT_MAX: usize = 10;

/// One shelf per console, then the recently played shelf and the
/// favourites shelf, empty until `set_recent` and `set_favourites` fill them.
fn shelves_of(carts: Vec<Cart>) -> Vec<(ShelfKey, Shelf)> {
    let mut rows: Vec<(Platform, Vec<Cart>)> =
        Platform::ALL.iter().map(|p| (*p, Vec::new())).collect();
    for cart in carts {
        if let Some((_, row)) = rows.iter_mut().find(|(p, _)| *p == cart.platform) {
            row.push(cart);
        }
    }
    rows.into_iter()
        .map(|(platform, carts)| (ShelfKey::Platform(platform), Shelf::new(carts)))
        .chain([
            (ShelfKey::Picks, Shelf::new(Vec::new())),
            (ShelfKey::Recent, Shelf::new(Vec::new())),
            (ShelfKey::Favourites, Shelf::new(Vec::new())),
        ])
        .collect()
}

fn favourites_path(root: &Path) -> PathBuf {
    root.join("Config").join("favourites.txt")
}

/// The favourite roms, newest first.
pub fn read_favourites(root: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(favourites_path(root))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn hidden_path(root: &Path) -> PathBuf {
    root.join("Config").join("hidden.txt")
}

/// The roms hidden from the shelves.
pub fn read_hidden(root: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(hidden_path(root))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn recent_path(root: &Path) -> PathBuf {
    root.join("Config").join("recent.txt")
}

/// The roms played last, newest first, as Shelf wrote them down.
pub fn read_recent(root: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(recent_path(root))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

impl App {
    pub fn new(carts: Vec<Cart>) -> Self {
        let all_carts = carts.clone();
        let shelves = shelves_of(carts);
        let shelf_at = shelves
            .iter()
            .position(|(_, s)| !s.carts.is_empty())
            .unwrap_or(0);
        let state_writer = crate::state_writer::StateWriter::spawn();
        let card_failing = state_writer.card_failing();
        let (shelf_count, details_loaded) = (shelves.len(), vec![false; shelves.len()]);
        let mut app = App {
            details_loaded,
            details_rx: None,
            pick_facts: HashMap::new(),
            picks_rx: None,
            shelf_sorted: vec![false; shelf_count],
            section_labels: vec![Vec::new(); shelf_count],
            sign_section: None,
            sign_at: 0,
            section_faces: None,
            store_art: None,
            face_epoch: 0,
            state_writer,
            phase: Phase::Shelf,
            shelves,
            all_carts,
            hidden: Vec::new(),
            fresh: false,
            about_back: false,
            member_face: None,
            receipts: Vec::new(),
            slip: None,
            receipt_face: None,
            shelf_at,
            play_held: None,
            refusal: None,
            blow: None,
            blow_rom: None,
            played: Played::new(),
            rentals: Rentals::new(),
            nextui_plays: Rentals::new(),
            play_times: shelf_store::PlayTimes::new(),
            nextui_times: shelf_store::PlayTimes::new(),
            resume_lookup: None,
            saved: HashMap::new(),
            saved_face: None,
            note_face: None,
            stat_tags: HashMap::new(),
            help: false,
            spin: None,
            box_view: None,
            peek: None,
            peek_step: None,
            peek_rom: None,
            box_art: None,
            box_face: None,
            box_back: None,
            box_back_face: None,
            help_faces: Vec::new(),
            cart_hints: None,
            shutdown_faces: Vec::new(),
            parked: None,
            restarting: false,
            artwork: None,
            artwork_net: None,
            artwork_need: (0, 0, 0, None),
            artwork_gaps: Vec::new(),
            artwork_faces: None,
            act_at: 0,
            root: None,
            state: ShelfState::default(),
            vol_before: Vec::new(),
            sfx: None,
            clock_faces: None,
            quick_menu_faces: None,
            quick_clock_faces: None,
            sticker_face: None,
            wallpaper: None,
            battery_percent: shelf_ui::Printed::default(),
            bolt: None,
            shelf_platform: shelf_ui::Printed::default(),
            store_ticket: None,
            game_name: None,
            details: HashMap::new(),
            details_line: None,
            back_details_face: None,
            info: None,
            info_face: None,
            name_pending: false,
            slot_letter: None,
            shelf_named: None,
            shelf_clock: shelf_ui::Printed::default(),
            hud: Hud::new(),
            clock: 0.0,
            power: None,
            card_failing,
            card_read_only: false,
            card_check_at: 0,
            battery_at: BATTERY_POLL_MS,
            charge_at: CHARGE_POLL_MS,
            battery: None,
            powering_off: false,
            exiting: false,
            handed: None,
            sleep_wanted: false,
            screen_timeout: None,
            idle_ms: 0.0,
            shed_at: None,
        };
        app.resort_all();
        app
    }

    /// Build the app on a thread of its own, so the scan and the state
    /// reads go on while the screen is being set up.
    pub fn boot_in_background(root: &Path) -> Booting {
        let owned = root.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("shelf-boot".into())
            .spawn(move || App::boot(&owned));
        match spawned {
            Ok(handle) => Booting::Thread(handle, root.to_path_buf()),
            Err(_) => Booting::Inline(root.to_path_buf()),
        }
    }

    pub fn boot(root: &Path) -> Self {
        crate::root::ensure(root);
        shelf_ui::set_theme(Theme::read(root));
        crate::boot_time::mark("root and theme");
        let roms = crate::handoff::roms();
        let only = crate::handoff::only();
        let carts = match (&only, roms.as_deref()) {
            (Some(only), Some(roms)) => {
                let carts = shelf_store::scan_listed(root, roms, only);
                eprintln!(
                    "shelf: scan: {} games from the lists in {}",
                    carts.len(),
                    only.display()
                );
                carts
            }
            _ => scan_in(root, roms.as_deref()).unwrap_or_default(),
        };
        crate::boot_time::mark("scan");
        warm_headers(&carts);
        crate::boot_time::mark("headers");
        let mut app = App::new(carts);
        app.warm_shapes();
        app.set_recent(&read_recent(root));
        app.set_favourites(&read_favourites(root));
        app.root = Some(root.to_path_buf());
        app.state = read_shelf_state(root);
        app.hidden = read_hidden(root);
        if !app.hidden.is_empty() {
            app.apply_hidden();
        }
        app.played = read_played(root);
        app.rentals = read_rentals(root);
        app.play_times = shelf_store::read_play_times(root);
        let back = shelf_store::finish_playing(root, monotonic_secs());
        app.apply_sort_state();
        app.receipts = shelf_store::read_receipts(root);
        let shared = crate::handoff::shared_userdata();
        let mut rows = match (&shared, &roms) {
            (Some(shared), Some(roms)) => crate::game_logs::nextui_rows(shared, roms),
            _ => Vec::new(),
        };
        crate::game_logs::cut_at_power_offs(&mut rows, &shelf_store::read_power_offs(root));
        let back = back.is_some_and(|session| app.take_back(session, &rows));
        let windows: Vec<crate::game_logs::ShelfWindow> = app
            .receipts
            .iter()
            .map(|r| (r.rom.clone(), r.rented, r.returned))
            .collect();
        app.nextui_plays = crate::game_logs::nextui_plays(&rows, &windows);
        app.nextui_times = crate::game_logs::nextui_play_times(&rows, &windows);
        if let (Some(shared), Some(roms)) = (shared, roms) {
            app.set_screen_timeout(crate::handoff::screen_timeout(&shared));
            crate::boot_time::mark("game logs");
            app.set_resume_lookup(shared, roms);
        }
        app.read_pick_facts();
        app.refresh_picks();
        app.settle_member_since();
        if back {
            app.print_receipt();
        }
        app.refresh_dust();
        app.start();
        app
    }

    /// Start building the masks every shape on the shelves is drawn with,
    /// off to the side, so the first cart faces are not held up by them.
    fn warm_shapes(&self) {
        let mut shapes: Vec<shelf_ui::CartShape> = Vec::new();
        for (_, shelf) in &self.shelves {
            for shape in shelf.shapes() {
                if !shapes.contains(shape) {
                    shapes.push(*shape);
                }
            }
        }
        for shape in shapes {
            let _ = std::thread::Builder::new()
                .name("shelf-warm".into())
                .spawn(move || shelf_ui::warm_shape(shape));
        }
    }

    fn start(&mut self) {
        self.restore_places();
        let seated = if self.single_cart() {
            Some((self.shelf_at, 0))
        } else {
            let stem = self.state.cart.clone();
            let platform = self.state.cart_platform;
            stem.and_then(|stem| self.seat_of(&stem, platform))
        };
        self.phase = Phase::Shelf;
        match seated {
            Some((at, i)) => {
                self.shelf_at = at;
                self.shelf_mut().select(i);
            }
            None => {
                self.state.cart = None;
                self.state.cart_platform = None;
                self.name_pending = true;
            }
        }
    }

    fn shelf(&self) -> &Shelf {
        &self.shelves[self.shelf_at].1
    }

    /// Put each console's shelf back on the game it was last left on.
    fn restore_places(&mut self) {
        let places = &self.state.places;
        for (key, shelf) in &mut self.shelves {
            let ShelfKey::Platform(p) = key else {
                continue;
            };
            let Some((_, stem)) = places.iter().find(|(q, _)| q == p) else {
                continue;
            };
            if let Some(i) = shelf.carts.iter().position(|c| &c.stem == stem) {
                shelf.select(i);
            }
        }
    }

    /// Note the game the shelf being left is on, for `restore_places`.
    fn note_place(&mut self) {
        let ShelfKey::Platform(p) = self.shelves[self.shelf_at].0 else {
            return;
        };
        let Some(stem) = self
            .shelf()
            .carts
            .get(self.shelf().index)
            .map(|c| c.stem.clone())
        else {
            return;
        };
        let places = &mut self.state.places;
        match places.iter_mut().find(|(q, _)| *q == p) {
            Some(place) => place.1 = stem,
            None => places.push((p, stem)),
        }
    }

    fn shelf_mut(&mut self) -> &mut Shelf {
        &mut self.shelves[self.shelf_at].1
    }

    fn switch_shelf(&mut self, by: i32) {
        let Some(to) = self.next_shelf(by) else {
            return;
        };
        self.switch_shelf_to(to);
    }

    fn switch_shelf_to(&mut self, to: usize) {
        self.note_place();
        self.shelf_mut().release_hold();
        self.play_held = None;
        self.shelf_at = to;
        self.shelf_platform = shelf_ui::Printed::default();
        self.slot_letter = None;
        self.shelf_named = None;
        self.name_pending = true;
    }

    fn jump_letter(&mut self, dir: i32) {
        let before = self.shelf().index;
        match dir > 0 {
            true => self.shelf_mut().jump_next_letter(),
            false => self.shelf_mut().jump_prev_letter(),
        }
        if self.shelf().index == before {
            return;
        }
        let Some(letter) = self.jumped_to() else {
            return;
        };
        if self.slot_letter.as_ref() != Some(&letter) {
            self.shelf_platform = shelf_ui::Printed::default();
        }
        self.slot_letter = Some(letter);
        self.shelf_named = None;
        self.name_pending = true;
    }

    /// Brings the shelf's name back into the slot whenever it is missing: on
    /// coming back to the shelf, and once a jumped-to letter has faded.
    fn keep_shelf_named(&mut self) {
        if !self.on_shelf() || self.name_pending {
            return;
        }
        let faded = self.shelf_named.is_some_and(|at| {
            self.now().saturating_sub(at) >= SLOT_NAME_IN_MS + SLOT_NAME_HOLD_MS + SLOT_NAME_OUT_MS
        });
        if self.slot_letter.is_some() && faded {
            self.slot_letter = None;
            self.shelf_platform = shelf_ui::Printed::default();
            self.shelf_named = None;
        }
        if self.shelf_named.is_none() {
            self.name_pending = true;
        }
    }

    fn slot_name_alpha(&self) -> f32 {
        let Some(at) = self.shelf_named else {
            return 0.0;
        };
        let t = self.now().saturating_sub(at);
        let level = if t < SLOT_NAME_IN_MS {
            t as f32 / SLOT_NAME_IN_MS as f32
        } else if t < SLOT_NAME_IN_MS + SLOT_NAME_HOLD_MS || self.slot_letter.is_none() {
            // The shelf's own name stays up; only a jumped-to letter fades.
            1.0
        } else {
            let out = t - SLOT_NAME_IN_MS - SLOT_NAME_HOLD_MS;
            1.0 - (out as f32 / SLOT_NAME_OUT_MS as f32).min(1.0)
        };
        SLOT_NAME_ALPHA * ease(level)
    }

    fn next_shelf(&self, by: i32) -> Option<usize> {
        let n = self.shelves.len() as i32;
        (1..n)
            .map(|step| (self.shelf_at as i32 + by * step).rem_euclid(n) as usize)
            .find(|at| !self.shelves[*at].1.carts.is_empty())
    }

    fn seat_of(&self, stem: &str, platform: Option<Platform>) -> Option<(usize, usize)> {
        self.shelves
            .iter()
            .enumerate()
            .filter(|(_, (key, _))| match key {
                ShelfKey::Platform(p) => platform.is_none_or(|want| *p == want),
                ShelfKey::Picks | ShelfKey::Recent | ShelfKey::Favourites => false,
            })
            .find_map(|(at, (_, shelf))| {
                shelf
                    .carts
                    .iter()
                    .position(|c| c.stem == stem)
                    .map(|i| (at, i))
            })
    }

    pub fn confirm_clock(&mut self) {
        let Phase::SetClock {
            picker,
            seed,
            from_menu,
        } = &self.phase
        else {
            return;
        };
        let (moved, offset, from_menu) = (picker.secs() - *seed, picker.offset_min(), *from_menu);
        let utc = self.utc_secs() + moved;
        if let Some(power) = &mut self.power {
            power.set_clock(utc);
        }
        self.state.utc_offset_min = offset as i16;
        self.state.clock_set = true;
        self.refresh_dust();
        self.persist();
        if from_menu {
            self.phase = Phase::QuickMenu {
                row: QuickRow::DateTime,
            };
        } else {
            self.start();
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn wall_secs(&self) -> i64 {
        self.utc_secs() + i64::from(self.state.utc_offset_min) * 60
    }

    fn utc_secs(&self) -> i64 {
        self.power.as_ref().map_or_else(system_secs, |p| p.now())
    }

    pub fn picker(&self) -> Option<&ClockPicker> {
        match &self.phase {
            Phase::SetClock { picker, .. } => Some(picker),
            _ => None,
        }
    }

    pub fn quick_menu(&self) -> Option<QuickRow> {
        match self.phase {
            Phase::QuickMenu { row } => Some(row),
            _ => None,
        }
    }

    pub fn quick_value(&self, row: QuickRow) -> Option<QuickValue> {
        match row {
            QuickRow::Crt => Some(QuickValue::flag(self.state.crt)),
            QuickRow::Dusty => Some(QuickValue::flag(self.state.dusty)),
            QuickRow::Resume => Some(QuickValue::flag(self.state.resume)),
            QuickRow::ShowHidden => Some(QuickValue::flag(self.state.show_hidden)),
            QuickRow::PrintReceipts => Some(QuickValue::flag(self.state.receipts)),
            QuickRow::Display => Some(QuickValue::display(self.state.display)),
            QuickRow::View => Some(QuickValue::view(self.view_carts())),
            QuickRow::Sort => Some(QuickValue::sort(self.state.sort)),
            _ => None,
        }
    }

    pub fn set_quick_menu_faces(&mut self, faces: QuickMenuFaces) {
        self.quick_menu_faces = Some(faces);
    }

    pub fn set_quick_clock_faces(&mut self, dim: (TexId, u32, u32), lit: (TexId, u32, u32)) {
        self.quick_clock_faces = Some([dim, lit]);
    }

    pub fn set_sticker_face(&mut self, face: TexId) {
        self.sticker_face = Some(face);
    }

    pub fn set_clock_faces(&mut self, line: TexId, hint: TexId) {
        self.clock_faces = Some((line, hint));
    }

    pub fn set_cart_shadow(&mut self, face: TexId) {
        for (_, shelf) in &mut self.shelves {
            shelf.set_shadow(face);
        }
    }

    pub fn set_gb_cart_shadows(&mut self, notched: TexId, rounded: TexId) {
        for (_, shelf) in &mut self.shelves {
            shelf.set_gb_shadow(GbShell::Notched, notched);
            shelf.set_gb_shadow(GbShell::Rounded, rounded);
        }
    }

    pub fn set_console_cart_shadow(&mut self, console: shelf_ui::Console, face: TexId) {
        for (_, shelf) in &mut self.shelves {
            shelf.set_console_shadow(console, face);
        }
    }

    pub fn set_wallpaper(&mut self, face: TexId) {
        self.wallpaper = Some(face);
    }

    pub fn set_bolt_face(&mut self, bolt: TexId) {
        self.bolt = Some(bolt);
    }

    pub fn set_shelf_platform_face(&mut self, face: TexId, w: u32) {
        self.shelf_platform = shelf_ui::Printed::new(face, w);
    }

    /// The name to print above the cart in the middle of the shelf.
    pub fn wanted_game_name(&self) -> Option<String> {
        if !self.on_shelf() {
            return None;
        }
        let shelf = self.shelf();
        shelf
            .carts
            .get(shelf.index)
            .map(|c| shelf_ui::game_name(&c.stem))
    }

    pub fn set_game_name_face(&mut self, name: String, face: TexId) {
        self.game_name = Some((name, face));
    }

    fn draw_game_name(&self, out: &mut Vec<Draw>) {
        let (Some((name, tex)), Some(want)) = (&self.game_name, self.wanted_game_name()) else {
            return;
        };
        let shelf = self.shelf();
        let Some(cart) = shelf.carts.get(shelf.index) else {
            return;
        };
        if *name != want {
            return;
        }
        let (y, top) = self.name_place(cart);
        out.push(Draw::Tex {
            x: (OUT_W - shelf_ui::NAME_W) as f32 / 2.0,
            y,
            w: shelf_ui::NAME_W as f32,
            h: shelf_ui::NAME_H as f32,
            tex: *tex,
            alpha: 1.0,
        });
        // The details go snug under the name, if there is room for them
        // above the cart.
        let line_y = y + shelf_ui::DETAILS_DROP;
        if let Some((rom, line)) = &self.details_line {
            if *rom == cart.rom
                && self
                    .details
                    .get(&cart.rom)
                    .is_some_and(|d| d.is_some() || self.plain())
                && line_y + shelf_ui::DETAILS_H as f32 <= top
            {
                out.push(Draw::Tex {
                    x: (OUT_W - shelf_ui::DETAILS_W) as f32 / 2.0,
                    y: line_y,
                    w: shelf_ui::DETAILS_W as f32,
                    h: shelf_ui::DETAILS_H as f32,
                    tex: *line,
                    alpha: 1.0,
                });
            }
        }
    }

    /// Where the game's name is printed, its top, and the top of what
    /// stands under it, the cart or the box.
    fn name_place(&self, cart: &Cart) -> (f32, f32) {
        match self.state.display {
            DisplayMode::Carts => {
                // Near the top of the screen, up in the band a 4:3 panel
                // adds above the picture, so the details under it have room
                // above even a tall cart.
                let top = shelf_ui::rest_y(shelf_ui::cart_box(cart.platform).1 as f32);
                (NAME_TOP - shelf_ui::band(), top)
            }
            DisplayMode::Store => (shelf_ui::store_name_top(), shelf_ui::store_ceiling()),
            // Where the cart shelf prints it, the boxes kept below.
            DisplayMode::Boxes => (NAME_TOP - shelf_ui::band(), boxes_ceiling()),
        }
    }

    /// The shelf has rested on `rom`: read what is known of the game, once.
    pub fn load_details_of(&mut self, rom: &Path) {
        if self.details.contains_key(rom) {
            return;
        }
        let loaded = match (&self.root, self.selected_cart()) {
            (Some(root), Some(cart)) if cart.rom == rom => details_for(root, cart),
            _ => return,
        };
        self.details.insert(rom.to_path_buf(), loaded);
    }

    /// What is known of the game that has been looked at, `rom`, if
    /// anything is.
    pub fn details_of(&self, rom: &Path) -> Option<&Details> {
        self.details.get(rom)?.as_ref()
    }

    /// The details to print under the middle cart's name, with the rom
    /// they are of.
    pub fn wanted_details_line(&self) -> Option<(PathBuf, Details)> {
        if !self.on_shelf() {
            return None;
        }
        let cart = self.selected_cart()?;
        // The original theme has no store tags, so a game LaunchBox knows
        // nothing of still gets a line for its plays.
        let details = match self.details.get(&cart.rom)? {
            Some(d) => d.clone(),
            None if self.plain() => Details::default(),
            None => return None,
        };
        Some((cart.rom.clone(), details))
    }

    /// In the original theme, which has no store tags, the middle game's
    /// plays at the end of the line under its name: "PLAYED 12× · LAST 3
    /// OCT", or NEW for a game never played.
    pub fn details_line_plays(&self) -> Option<String> {
        if !self.plain() {
            return None;
        }
        let cart = self.selected_cart()?;
        let offset = i64::from(self.state.utc_offset_min) * 60;
        let sticker = self
            .rental_of(&cart.rom)
            .and_then(|r| shelf_ui::RentalSticker::new(r.times, r.last + offset));
        Some(match sticker {
            Some(s) => {
                let (played, last) = shelf_ui::rental_lines(&s.worded(true));
                let time = shelf_ui::play_time_text(self.play_time_of(&cart.rom))
                    .map_or(String::new(), |t| format!(" \u{b7} {t}"));
                format!(
                    "{played}{time} \u{b7} {}",
                    last.replacen("LAST PLAYED", "LAST", 1)
                )
            }
            None => "NEW".to_string(),
        })
    }

    pub fn set_details_line_face(&mut self, rom: PathBuf, face: TexId) {
        self.details_line = Some((rom, face));
    }

    /// What box view needs made to show the game's details on the box
    /// back, once the pictures it goes with are in.
    pub fn back_details_key(&self) -> Option<BackKey> {
        let cart = self.selected_cart()?;
        // The size the back is drawn at: the box held up in box view, or on
        // the box shelf the box where it stands.
        let front: (u32, u32) = match (self.box_view, self.peek) {
            (Some(_), _) => self
                .box_face
                .as_ref()
                .filter(|f| Some(&f.0) == self.box_art.as_ref())
                .map(|f| (f.2, f.3))?,
            (None, Some(peek)) if peek.on_back(self.now()) => {
                let (_, _, w, h) = self.shelf().boxes_selected_rect(boxes_ceiling())?;
                (w.round() as u32, h.round() as u32)
            }
            _ => return None,
        };
        self.details_of(&cart.rom)?;
        let stem = cart.stem.clone();
        let panel = shelf_ui::slot_colours(Some(cart.platform))[0];
        let offset = i64::from(self.state.utc_offset_min) * 60;
        let rental = self
            .rental_of(&cart.rom)
            .and_then(|r| shelf_ui::RentalSticker::new(r.times, r.last + offset))
            .map(|r| r.worded(self.plain()));
        if self.box_back.is_none() {
            return Some(BackKey {
                stem,
                strip: false,
                w: front.0,
                h: front.1,
                panel,
                rental,
            });
        }
        let scan = self
            .box_back_face
            .as_ref()
            .filter(|f| Some(&f.0) == self.box_back.as_ref())?;
        let fit = (front.0 as f32 / scan.2 as f32).min(front.1 as f32 / scan.3 as f32);
        Some(BackKey {
            stem,
            strip: true,
            w: (scan.2 as f32 * fit).round() as u32,
            h: (scan.3 as f32 * fit).round() as u32,
            panel,
            rental,
        })
    }

    /// The details of the selected game, to make the box back from.
    pub fn back_details(&self) -> Option<&Details> {
        self.details_of(&self.selected_cart()?.rom)
    }

    pub fn set_back_details_face(&mut self, key: BackKey, face: TexId, w: u32, h: u32) {
        self.back_details_face = Some((key, face, w, h));
    }

    pub fn clear_shelf_platform(&mut self) {
        self.shelf_platform = shelf_ui::Printed::default();
        self.store_ticket = None;
    }

    pub fn set_store_ticket_face(&mut self, face: TexId, w: u32, h: u32) {
        self.store_ticket = Some((face, w, h));
    }

    pub fn slot_text(&self) -> Option<String> {
        match &self.slot_letter {
            Some(letter) => Some(letter.clone()),
            None => self.shelf_platform_name().map(str::to_string),
        }
    }

    pub fn shelf_platform_name(&self) -> Option<&'static str> {
        self.next_shelf(1)?;
        Some(self.shelves[self.shelf_at].0.name_in(self.plain()))
    }

    pub fn set_battery_percent_face(&mut self, face: TexId, w: u32) {
        self.battery_percent = shelf_ui::Printed::new(face, w);
    }

    pub fn set_shelf_clock_face(&mut self, face: TexId, w: u32) {
        self.shelf_clock = shelf_ui::Printed::new(face, w);
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn carts(&self) -> impl Iterator<Item = &Cart> {
        self.shelves
            .iter()
            .filter(|(key, _)| !key.is_list())
            .flat_map(|(_, shelf)| shelf.carts.iter())
    }

    pub fn seated_cart(&self) -> Option<&Cart> {
        let Phase::Inserting { cart, .. } = &self.phase else {
            return None;
        };
        self.shelf().carts.iter().find(|c| c.stem == *cart)
    }

    pub fn single_cart(&self) -> bool {
        self.carts().count() == 1
    }

    pub fn set_faces(&mut self, faces: Vec<TexId>) {
        let mut faces = faces.into_iter();
        for (_, shelf) in &mut self.shelves {
            let n = shelf.carts.len();
            shelf.set_faces(faces.by_ref().take(n).collect());
        }
    }

    /// Which picture of each game is drawn now.
    pub fn face_kind(&self) -> FaceKind {
        match self.state.display {
            DisplayMode::Carts => FaceKind::Cart,
            DisplayMode::Store if self.state.store_carts => FaceKind::Cart,
            DisplayMode::Store | DisplayMode::Boxes => FaceKind::Tall,
        }
    }

    fn has_face(&self, shelf: usize, index: usize, kind: FaceKind) -> bool {
        let shelf = &self.shelves[shelf].1;
        match kind {
            FaceKind::Cart => shelf.face(index).is_some(),
            FaceKind::Box => shelf.box_face(index).is_some(),
            FaceKind::Tall => shelf.tall_face(index).is_some(),
        }
    }

    pub fn face_wants(&self) -> Vec<FaceWant> {
        let kind = self.face_kind();
        let here = self.shelf();
        let screen = here.on_screen();
        let mut wants: Vec<(usize, usize)> = Vec::new();
        // A spin's landing place is known from the press: its pictures are
        // made while the shelf runs, before those flying past.
        let landing = self.spin_landing();
        wants.extend(landing.iter().map(|i| (self.shelf_at, *i)));
        wants.extend(screen.iter().map(|i| (self.shelf_at, *i)));
        let ahead = here
            .around(if kind == FaceKind::Cart {
                FACE_AHEAD
            } else {
                TALL_AHEAD
            })
            .into_iter()
            .filter(|i| !screen.contains(i))
            .map(|i| (self.shelf_at, i));
        if kind == FaceKind::Cart {
            // The cart shelves stack, so the next ones show.
            for at in self.neighbour_shelves() {
                wants.extend(self.shelves[at].1.on_screen().into_iter().map(|i| (at, i)));
            }
            wants.extend(ahead);
        } else {
            // The store and the box shelf show one shelf: the next ones
            // only need their middle boxes ready to step to, after this
            // shelf's look-ahead.
            wants.extend(ahead);
            for at in self.neighbour_shelves() {
                wants.extend(self.shelves[at].1.around(1).into_iter().map(|i| (at, i)));
            }
        }
        let mut out: Vec<FaceWant> = Vec::new();
        // On the box shelf the middle game's cart comes out of its box: it
        // is wanted first, once the shelf stops there. While it runs, the
        // carts of the boxes going past would hold up the boxes themselves.
        let middle = here.index;
        if kind == FaceKind::Tall
            && self.shelf_still()
            && !self.has_face(self.shelf_at, middle, FaceKind::Cart)
        {
            out.push(FaceWant {
                shelf: self.shelf_at,
                index: middle,
                on_screen: true,
                kind: FaceKind::Cart,
            });
        }
        let mut seen = std::collections::HashSet::new();
        out.extend(
            wants
                .into_iter()
                .filter(|w| seen.insert(*w))
                .filter(|(at, i)| !self.has_face(*at, *i, kind))
                .map(|(shelf, index)| FaceWant {
                    shelf,
                    index,
                    on_screen: shelf == self.shelf_at && screen.contains(&index),
                    kind,
                }),
        );
        out
    }

    /// Whether the shelf has stopped: no spin, no direction held, and the
    /// scroll come to rest.
    pub fn shelf_still(&self) -> bool {
        self.spin.is_none() && self.shelf().settled()
    }

    /// Every game's pictures of the kinds now drawn, nearest the selection
    /// first and this shelf before the others: read once in the background
    /// so that the card has them to hand when the shelf gets there.
    pub fn warm_order(&self) -> Vec<(FaceKind, Cart)> {
        let kind = self.face_kind();
        let mut kinds = vec![kind];
        if kind == FaceKind::Tall {
            kinds.push(FaceKind::Cart);
        }
        let mut order = vec![self.shelf_at];
        order.extend((0..self.shelves.len()).filter(|at| *at != self.shelf_at));
        let mut out = Vec::new();
        for at in order {
            let shelf = &self.shelves[at].1;
            if self.shelves[at].0.is_list() && at != self.shelf_at {
                continue;
            }
            let n = shelf.carts.len() as i32;
            for i in shelf.around(n / 2 + 1) {
                for k in &kinds {
                    out.push((*k, shelf.carts[i].clone()));
                }
            }
        }
        out
    }

    /// The places on this shelf around where a spin will come to rest,
    /// nearest first; none when not spinning.
    fn spin_landing(&self) -> Vec<usize> {
        let Some(spin) = self.spin else {
            return Vec::new();
        };
        let n = self.shelf().carts.len() as i32;
        let mut out = Vec::new();
        for step in 0..=3 {
            for off in [step, -step] {
                let i = (spin.target() as i32 + off).rem_euclid(n.max(1)) as usize;
                if !out.contains(&i) {
                    out.push(i);
                }
            }
        }
        out
    }

    pub fn face_cart(&self, shelf: usize, index: usize) -> Option<&Cart> {
        self.shelves.get(shelf)?.1.carts.get(index)
    }

    pub fn set_cart_face(&mut self, shelf: usize, index: usize, tex: TexId) -> Option<TexId> {
        self.shed_at = None;
        self.shelves.get_mut(shelf)?.1.set_face(index, tex)
    }

    /// The box front made for a cart on the store shelf.
    pub fn set_store_face(
        &mut self,
        shelf: usize,
        index: usize,
        tex: TexId,
        w: u32,
        h: u32,
    ) -> Option<TexId> {
        self.shed_at = None;
        self.shelves
            .get_mut(shelf)?
            .1
            .set_box_face(index, tex, w, h)
    }

    /// The box front made for a cart on the box shelf.
    pub fn set_tall_face(
        &mut self,
        shelf: usize,
        index: usize,
        tex: TexId,
        w: u32,
        h: u32,
    ) -> Option<TexId> {
        self.shed_at = None;
        self.shelves
            .get_mut(shelf)?
            .1
            .set_tall_face(index, tex, w, h)
    }

    /// How many times the shelves have changed under the faces being made.
    pub fn face_epoch(&self) -> u32 {
        self.face_epoch
    }

    pub fn shed_faces(&mut self) -> Vec<TexId> {
        let at = (self.shelf_at, self.shelf().index);
        if self.shed_at == Some(at) {
            return Vec::new();
        }
        self.shed_at = Some(at);
        let neighbours = self.neighbour_shelves();
        let kind = self.face_kind();
        let landing = self.spin_landing();
        let mut shed = Vec::new();
        for (at, (_, shelf)) in self.shelves.iter_mut().enumerate() {
            // The pictures in use are kept for a good way each side; the
            // other kind only for what is on the screen, to come back to.
            let (near, screen) = if at == self.shelf_at {
                let keep = if kind == FaceKind::Cart {
                    FACE_KEEP
                } else {
                    TALL_KEEP
                };
                let mut near = shelf.around(keep);
                near.extend(&landing);
                let mut screen = shelf.on_screen();
                screen.extend(&landing);
                (near, screen)
            } else if neighbours.contains(&at) {
                let near = match kind {
                    FaceKind::Cart => shelf.on_screen(),
                    _ => shelf.around(1),
                };
                (near, Vec::new())
            } else {
                (Vec::new(), Vec::new())
            };
            let (carts, boxes, talls) = match kind {
                FaceKind::Cart => (&near, &screen, &screen),
                FaceKind::Box => (&screen, &near, &screen),
                FaceKind::Tall => (&screen, &screen, &near),
            };
            for i in 0..shelf.carts.len() {
                if !carts.contains(&i) {
                    shed.extend(shelf.take_face(i));
                }
                if !boxes.contains(&i) {
                    shed.extend(shelf.take_box_face(i));
                }
                if !talls.contains(&i) {
                    shed.extend(shelf.take_tall_face(i));
                }
            }
        }
        shed
    }

    fn neighbour_shelves(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for by in [1, -1] {
            if let Some(at) = self.next_shelf(by) {
                if !out.contains(&at) {
                    out.push(at);
                }
            }
        }
        out
    }

    pub fn take_sfx(&mut self) -> Option<Sfx> {
        self.sfx.take()
    }

    /// Go to sleep, as a tap on power does, once no button has been pressed
    /// for NextUI's screen timeout. A frame's time counts for at most a
    /// second, so a long gap, such as the time asleep, does not count.
    fn count_idle(&mut self, dt: f32) {
        let Some(timeout) = self.screen_timeout else {
            return;
        };
        self.idle_ms += (dt as f64 * 1000.0).clamp(0.0, 1000.0);
        let quiet = matches!(
            self.phase,
            Phase::Shelf | Phase::QuickMenu { .. } | Phase::About
        );
        if !quiet || self.powering_off {
            self.idle_ms = 0.0;
            return;
        }
        if self.idle_ms >= timeout as f64 {
            self.idle_ms = 0.0;
            self.sleep_wanted = true;
        }
    }

    /// Sleep by itself after `timeout` with no button pressed, or never.
    pub fn set_screen_timeout(&mut self, timeout: Option<Millis>) {
        self.screen_timeout = timeout.filter(|t| *t > 0);
    }

    /// Whether a tap on the power button asked to sleep, clearing the ask.
    pub fn take_sleep(&mut self) -> bool {
        std::mem::take(&mut self.sleep_wanted)
    }

    /// Screen off, then suspend; returns how the suspend went. The caller
    /// waits for a power press itself when there was no way to suspend.
    pub fn suspend(&mut self) -> shelf_power::Suspend {
        let Some(power) = self.power.as_mut() else {
            return shelf_power::Suspend::Unavailable;
        };
        power.set_backlight(0);
        power.suspend()
    }

    /// Screen back on at the brightness it had.
    pub fn wake(&mut self) {
        let step = self.state.brightness;
        if let Some(power) = self.power.as_mut() {
            power.set_backlight(step);
        }
    }

    pub fn set_power(&mut self, mut power: Power) {
        power.set_backlight(self.state.brightness);
        let secs = power.now();
        if matches!(self.phase, Phase::SetClock { .. }) || secs < CLOCK_FLOOR {
            self.phase = clock_screen(secs, 0, false);
        }
        self.power = Some(power);
        self.refresh_dust();
        self.battery_at = self.now();
        self.charge_at = self.now();
    }

    /// X on the shelf: blow on the middle cart, for luck.
    fn blow_on_cart(&mut self, now: Millis) {
        // The box shelf's boxes are not blown on, but a cart taken out of
        // one is.
        let out = self.peek.is_some_and(|p| p.taken_out(now));
        if self.selected_stem().is_none() || (self.boxes() && !out) {
            return;
        }
        self.blow = Some(shelf_ui::Blow::started(now));
        self.blow_rom = self.selected_cart().map(|c| c.rom.clone());
    }

    /// Note that the game was played or blown on just now, so its cart
    /// is clean again.
    fn mark_played(&mut self, rom: &Path) {
        self.played.insert(rom.to_path_buf(), self.utc_secs());
        if let Some(root) = &self.root {
            if let Err(e) = write_played(root, &self.played) {
                eprintln!("shelf: played: {e}");
            }
        }
        self.refresh_dust();
    }

    /// Count a play of the game, for the rental sticker on its box.
    fn mark_rented(&mut self, rom: &Path) {
        let now = self.utc_secs();
        rent(&mut self.rentals, rom, now);
        if let Some(root) = &self.root {
            if let Err(e) = write_rentals(root, &self.rentals) {
                eprintln!("shelf: rentals: {e}");
            }
        }
    }

    /// How often and when last a game was played, if it ever was.
    /// How often `rom` has been played and when last: Shelf's own count
    /// and NextUI's log together.
    pub fn rental_of(&self, rom: &Path) -> Option<Rental> {
        crate::game_logs::combined(
            self.rentals.get(rom).copied(),
            self.nextui_plays.get(rom).copied(),
        )
    }

    /// Bring the card's member-since back to the first play anything
    /// recorded, or to now on a first start with the clock set.
    fn settle_member_since(&mut self) {
        let known = self
            .rentals
            .values()
            .chain(self.nextui_plays.values())
            .map(|r| r.last)
            .chain(self.played.values().copied())
            .filter(|t| *t > 0)
            .min();
        let now = self.utc_secs();
        let since = match (self.state.member_since, known) {
            (0, Some(t)) => t,
            (s, Some(t)) => s.min(t),
            (0, None) if self.state.clock_set => now,
            (s, None) => s,
        };
        if since != self.state.member_since {
            self.state.member_since = since;
            self.persist();
        }
    }

    /// Log `rom` coming back after `secs` of play, and print its receipt
    /// if receipts are on.
    /// A game back from NextUI: add its play time, which is what NextUI's
    /// timer counted between sleeps if it logged the session, else all the
    /// time it was out, and put its receipt on the spike for
    /// `print_receipt`.
    /// Returns whether there was a session to take back.
    fn take_back(
        &mut self,
        session: shelf_store::Session,
        rows: &[crate::game_logs::LogRow],
    ) -> bool {
        // Handed out before the Brick was last turned off, and not resumed
        // through Shelf: NextUI's log has it as one of its own plays.
        let boot = shelf_store::boot_id();
        if session.boot.is_some() && boot.is_some() && session.boot != boot {
            eprintln!("shelf: {} went out before a restart", session.rom.display());
            return false;
        }
        let returned = self.utc_secs();
        let logged = session
            .since
            .and_then(|since| crate::game_logs::session_secs(rows, &session.rom, since, returned));
        let Some(secs) = logged.or(session.secs) else {
            eprintln!("shelf: no believable time for {}", session.rom.display());
            return false;
        };
        eprintln!(
            "shelf: played {} for {secs} s ({:?} s out, NextUI logged {logged:?})",
            session.rom.display(),
            session.secs
        );
        *self.play_times.entry(session.rom.clone()).or_insert(0) += secs;
        if let Some(root) = &self.root {
            if let Err(e) = shelf_store::write_play_times(root, &self.play_times) {
                eprintln!("shelf: play time: {e}");
            }
        }
        self.receipts.insert(
            0,
            shelf_store::Receipt {
                rented: session.since.unwrap_or(returned - secs as i64),
                rom: session.rom,
                returned,
                secs,
                total: 0,
            },
        );
        true
    }

    /// Log the receipt `take_back` made, with the game's play time now
    /// NextUI's is known too, and print it if receipts are on.
    fn print_receipt(&mut self) {
        let Some(rom) = self.receipts.first().map(|r| r.rom.clone()) else {
            return;
        };
        let total = self.play_time_of(&rom);
        let receipt = &mut self.receipts[0];
        receipt.total = total;
        if let Some(root) = &self.root {
            if let Err(e) = shelf_store::append_receipt(root, receipt) {
                eprintln!("shelf: receipts: {e}");
            }
        }
        if self.state.receipts {
            self.slip = Some(Slip {
                at: None,
                torn: None,
            });
        }
    }

    pub fn slip(&self) -> Option<Slip> {
        self.slip
    }

    fn tear_slip(&mut self, now: Millis) {
        let Some(slip) = &mut self.slip else { return };
        // Not printing yet: there is nothing to tear.
        if slip.at.is_none_or(|at| now < at) {
            self.slip = None;
            return;
        }
        slip.torn.get_or_insert(now);
    }

    /// Start the receipt printing once the shelf shows, tear it off when
    /// it has been out long enough, and let it go once it is gone.
    fn settle_slip(&mut self, now: Millis) {
        let on_shelf = matches!(self.phase, Phase::Shelf);
        let Some(slip) = &mut self.slip else { return };
        if !on_shelf {
            return;
        }
        let at = *slip.at.get_or_insert(now + shelf_ui::SLIP_DELAY_MS);
        if slip.torn.is_none() && now >= at + shelf_ui::SLIP_PRINT_MS + shelf_ui::SLIP_STAY_MS {
            slip.torn = Some(now);
        }
        if slip.torn.is_some_and(|t| now >= t + shelf_ui::SLIP_TEAR_MS) {
            self.slip = None;
        }
    }

    fn spike_input(&mut self, at: usize, action: Action) {
        let last = self.receipts.len().saturating_sub(1);
        let at = match action {
            Action::GbaDown(Btn::Up) | Action::GbaDown(Btn::Left) => at.saturating_sub(1),
            Action::GbaDown(Btn::Down) | Action::GbaDown(Btn::Right) => (at + 1).min(last),
            Action::GbaDown(Btn::B) | Action::QuickMenu => {
                self.phase = Phase::QuickMenu {
                    row: QuickRow::Receipts,
                };
                return;
            }
            _ => return,
        };
        self.phase = Phase::Receipts { at };
    }

    /// The receipt to show: the one printing, or the one on top of the
    /// spike.
    pub fn receipt_wanted(&self) -> Option<ReceiptKey> {
        let at = match self.phase {
            Phase::Receipts { at } => at,
            Phase::Shelf if self.slip.is_some() => 0,
            _ => return None,
        };
        let plain = self.plain();
        let Some(r) = self.receipts.get(at) else {
            return Some((None, plain));
        };
        let cart = self.all_carts.iter().find(|c| c.rom == r.rom);
        let stem = r.rom.file_stem().map(|s| s.to_string_lossy().into_owned());
        let title = shelf_ui::game_name(&cart.map_or(stem.unwrap_or_default(), |c| c.stem.clone()));
        let console = cart.map_or_else(
            || {
                r.rom
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_uppercase())
                    .unwrap_or_default()
            },
            |c| c.platform.name().to_uppercase(),
        );
        let offset = i64::from(self.state.utc_offset_min) * 60;
        Some((
            Some(shelf_ui::ReceiptSlip {
                plain,
                console,
                title,
                rented: r.rented + offset,
                returned: r.returned + offset,
                due: r.due_back() + offset,
                secs: r.secs,
                total: r.total,
                number: (self.receipts.len() - at) as u32,
            }),
            plain,
        ))
    }

    pub fn set_receipt_face(&mut self, key: ReceiptKey, face: TexId) {
        self.receipt_face = Some((key, face));
    }

    pub fn receipt_face_key(&self) -> Option<&ReceiptKey> {
        self.receipt_face.as_ref().map(|(k, _)| k)
    }

    fn receipt_tex(&self) -> Option<TexId> {
        let wanted = self.receipt_wanted();
        self.receipt_face
            .as_ref()
            .filter(|(k, _)| Some(k) == wanted.as_ref())
            .map(|(_, t)| *t)
    }

    /// What the membership card says, while About shows it.
    pub fn member_card(&self) -> Option<shelf_ui::MemberCard> {
        if !matches!(self.phase, Phase::About) {
            return None;
        }
        let carts: Vec<&Cart> = self.all_carts.iter().collect();
        let plays: u32 = carts.iter().filter_map(|c| self.rental_of(&c.rom)).map(|r| r.times).sum();
        let secs: u64 = carts.iter().map(|c| self.play_time_of(&c.rom)).sum();
        let mut genres: HashMap<&str, u32> = HashMap::new();
        for c in &carts {
            let genre = self
                .pick_facts
                .get(&c.rom)
                .and_then(|f| f.1.first())
                .or_else(|| self.details_of(&c.rom).and_then(|d| d.genres.first()));
            if let (Some(g), Some(r)) = (genre, self.rental_of(&c.rom)) {
                *genres.entry(g.as_str()).or_insert(0) += r.times;
            }
        }
        let genre = genres
            .into_iter()
            .max_by_key(|(g, n)| (*n, std::cmp::Reverse(*g)))
            .map(|(g, _)| g.to_string());
        let offset = i64::from(self.state.utc_offset_min) * 60;
        Some(shelf_ui::MemberCard {
            plain: self.plain(),
            since: match self.state.member_since {
                0 => 0,
                t => t + offset,
            },
            plays,
            secs,
            genre,
            serial: String::new(),
        })
    }

    pub fn set_member_face(&mut self, card: shelf_ui::MemberCard, face: TexId) {
        self.member_face = Some((card, face));
    }

    pub fn member_face_card(&self) -> Option<&shelf_ui::MemberCard> {
        self.member_face.as_ref().map(|(c, _)| c)
    }

    /// How long `rom` has been played for, from Shelf and NextUI together.
    pub fn play_time_of(&self, rom: &Path) -> u64 {
        self.play_times.get(rom).copied().unwrap_or(0)
            + self.nextui_times.get(rom).copied().unwrap_or(0)
    }

    /// Where to look for NextUI's save-state slots: its shared userdata
    /// folder, and the Roms folder.
    pub fn set_resume_lookup(&mut self, shared_userdata: PathBuf, roms: PathBuf) {
        self.resume_lookup = Some((shared_userdata, roms));
        self.saved.clear();
    }

    pub fn set_saved_face(&mut self, face: (TexId, u32, u32)) {
        self.saved_face = Some(face);
    }

    pub fn set_note_face(&mut self, face: (TexId, u32, u32)) {
        self.note_face = Some(face);
    }

    /// The CONTINUE? note behind the middle box, at `rect`, poking out of
    /// its top, for a game with a save to go back to.
    fn draw_note_behind(&self, rect: Option<(f32, f32, f32, f32)>, out: &mut Vec<Draw>) {
        let (Some(face), Some((x, y, w, _))) = (self.note_face, rect) else {
            return;
        };
        if self.selected_saved() {
            shelf_ui::draw_note_behind(face, (x, y, w), 1.0, 1.0, out);
        }
    }

    /// The CONTINUE? note lying on the middle cart, once it is out of its
    /// box, for a game with a save.
    fn draw_note_on_cart(&self, out: &mut Vec<Draw>) {
        let (Some(face), Some((x, y, _, _))) = (self.note_face, self.shelf().selected_rect())
        else {
            return;
        };
        if self.selected_saved() {
            shelf_ui::draw_note_on(face, (x, y), out);
        }
    }

    /// Whether NextUI has a save state to continue for `rom`: its
    /// `.minui/<tag>/<rom file name>.txt` names the last slot used.
    fn has_save_slot(&self, rom: &Path) -> bool {
        let Some((shared, roms)) = &self.resume_lookup else {
            return false;
        };
        let Some(tag) = system_tag(rom, roms) else {
            return false;
        };
        let Some(name) = rom.file_name() else {
            return false;
        };
        let mut file = name.to_os_string();
        file.push(".txt");
        std::fs::metadata(shared.join(".minui").join(tag).join(file)).is_ok_and(|m| m.len() > 0)
    }

    /// Whether the middle cart's game has a save state Resume would load.
    pub fn selected_saved(&self) -> bool {
        self.state.resume
            && self
                .selected_cart()
                .is_some_and(|c| self.saved.get(&c.rom).copied().unwrap_or(false))
    }

    /// Look for the middle cart's save slot once per game.
    fn note_saved(&mut self) {
        if self.resume_lookup.is_none() || !self.on_shelf() {
            return;
        }
        let Some(rom) = self.selected_cart().map(|c| c.rom.clone()) else {
            return;
        };
        if !self.saved.contains_key(&rom) {
            let saved = self.has_save_slot(&rom);
            self.saved.insert(rom, saved);
        }
    }

    /// How dusty the middle cart is, 0 to 1, or 0 with the setting off.
    pub fn selected_dust(&self) -> f32 {
        let shelf = self.shelf();
        shelf.dust().get(shelf.index).copied().unwrap_or(0.0)
    }

    /// Work out how dusty each cart is, or none of them when the setting
    /// is off, and hand it to the shelves.
    fn refresh_dust(&mut self) {
        let now = self.utc_secs();
        let dusty = self.state.dusty;
        for (_, shelf) in &mut self.shelves {
            let dust = if dusty {
                shelf
                    .carts
                    .iter()
                    .map(|c| dust_amount(self.played.get(&c.rom).copied(), now))
                    .collect()
            } else {
                Vec::new()
            };
            shelf.set_dust(dust);
        }
    }

    /// The puff going on now, if any.
    /// Whether the button guide is open over the shelf.
    pub fn help_open(&self) -> bool {
        self.help
    }

    /// The guide's lines for the theme and view on show.
    pub fn help_lines(&self) -> Vec<shelf_ui::HelpLine> {
        shelf_ui::HelpLine::shown(self.store(), self.view_carts())
    }

    pub fn spin(&self) -> Option<shelf_ui::Spin> {
        self.spin
    }

    /// Select on the shelf: spin the shelf round to a cart picked at random.
    fn spin_for_a_game(&mut self, now: Millis) {
        let n = self.shelf().carts.len();
        if n < 2 {
            return self.refuse();
        }
        let from = self.shelf().index;
        let target = (from + 1 + (random_u64() % (n as u64 - 1)) as usize) % n;
        self.spin = Some(shelf_ui::Spin::new(now, from, target, n));
    }

    fn turn_spin(&mut self, now: Millis) {
        let Some(mut spin) = self.spin else { return };
        if !self.on_shelf() {
            self.spin = None;
            return;
        }
        for _ in 0..spin.due(now) {
            self.shelf_mut().right();
        }
        self.spin = (!spin.done()).then_some(spin);
    }

    pub fn box_view(&self) -> Option<shelf_ui::BoxView> {
        self.box_view
    }

    fn box_open(&self) -> bool {
        self.box_view.is_some_and(|b| b.is_open())
    }

    /// B on the shelf: turn the middle cart round to its box, or back.
    fn turn_box(&mut self, now: Millis) {
        if let Some(view) = self.box_view {
            self.box_view = Some(view.next(now));
            return;
        }
        let Some(path) = self.selected_cart().and_then(box_art_of) else {
            return self.refuse();
        };
        self.box_view = Some(shelf_ui::BoxView::opened(now));
        self.box_art = Some(path);
        self.box_back = match (&self.root, self.selected_cart()) {
            (Some(root), Some(cart)) => box_back_of(root, cart),
            _ => None,
        };
        if let Some(rom) = self.selected_cart().map(|c| c.rom.clone()) {
            self.load_details_of(&rom);
        }
    }

    fn selected_cart(&self) -> Option<&Cart> {
        let shelf = self.shelf();
        shelf.carts.get(shelf.index)
    }

    /// The box front and back of the middle cart, if its rom is `rom`, for
    /// loading ahead of B while the shelf rests on it.
    pub fn box_pictures_of(&self, rom: &Path) -> Option<(Option<PathBuf>, Option<PathBuf>)> {
        if self.box_view.is_some() || self.peek.is_some() {
            return None;
        }
        let cart = self.selected_cart().filter(|c| c.rom == rom)?;
        let back = self
            .root
            .as_deref()
            .and_then(|root| box_back_of(root, cart));
        // The box shelf draws its own fronts, and only wants the back.
        let front = box_art_of(cart).filter(|_| !self.boxes() && !self.store_boxes());
        Some((front, back))
    }

    /// The box picture the frontend should load for box view.
    pub fn wanted_box_art(&self) -> Option<&Path> {
        self.box_view?;
        self.box_art.as_deref()
    }

    pub fn set_box_face(&mut self, path: PathBuf, face: TexId, w: u32, h: u32) {
        self.box_face = Some((path, face, w, h));
    }

    /// The scan of the box's back the frontend should load, once the box
    /// is turning towards it.
    pub fn wanted_box_back(&self) -> Option<&Path> {
        let now = self.now();
        self.box_view
            .filter(|b| b.on_back(now))
            .or(self.peek.filter(|p| p.on_back(now)))?;
        self.box_back.as_deref()
    }

    pub fn set_box_back_face(&mut self, path: PathBuf, face: TexId, w: u32, h: u32) {
        self.box_back_face = Some((path, face, w, h));
    }

    /// The back scan would not load: draw a plain back instead.
    pub fn box_back_failed(&mut self) {
        self.box_back = None;
    }

    /// The box picture would not load: turn back to the cart.
    pub fn box_art_failed(&mut self) {
        self.box_view = None;
        self.box_art = None;
        self.refuse();
    }

    fn settle_box(&mut self, now: Millis) {
        if self.box_view.is_some_and(|b| b.shut(now)) || !self.on_shelf() {
            self.box_view = None;
        }
    }

    pub fn peek(&self) -> Option<shelf_ui::BoxView> {
        self.peek
    }

    /// A on the box shelf: open the middle box and take its cart out.
    fn open_peek(&mut self, now: Millis) {
        self.hold_box(shelf_ui::BoxView::peeked(now));
    }

    /// B on the box shelf: turn the middle box over to its back.
    fn flip_box(&mut self, now: Millis) {
        self.hold_box(shelf_ui::BoxView::flipped(now));
    }

    /// Take the middle box off the box shelf, to open or turn over.
    fn hold_box(&mut self, view: shelf_ui::BoxView) {
        let Some(cart) = self.selected_cart() else {
            return self.refuse();
        };
        let rom = cart.rom.clone();
        self.box_back = self
            .root
            .as_deref()
            .and_then(|root| box_back_of(root, cart));
        self.shelf_mut().release_hold();
        self.peek = Some(view);
        self.peek_step = None;
        self.load_details_of(&rom);
        self.peek_rom = Some(rom);
    }

    /// Put the peeked cart back and shut its box; then, with `step`, move
    /// along the shelf.
    fn close_peek(&mut self, now: Millis, step: Option<i32>) {
        self.play_held = None;
        self.peek = self.peek.map(|p| p.put_back(now));
        if step.is_some() {
            self.peek_step = step;
        }
    }

    /// Leave a peek at once, for something that takes the shelf away.
    fn drop_peek(&mut self) {
        self.peek = None;
        self.peek_step = None;
        self.peek_rom = None;
        self.play_held = None;
    }

    fn settle_peek(&mut self, now: Millis) {
        let moved = self.selected_cart().map(|c| &c.rom) != self.peek_rom.as_ref();
        if self.peek.is_some() && (!self.on_shelf() || moved) {
            return self.drop_peek();
        }
        if !self.peek.is_some_and(|p| p.put_away(now)) {
            return;
        }
        self.peek = None;
        self.peek_rom = None;
        match self.peek_step.take() {
            Some(by) if by < 0 => self.shelf_mut().left(),
            Some(_) => self.shelf_mut().right(),
            None => {}
        }
    }

    /// A button on the box shelf while a box is opened.
    fn peek_input(&mut self, action: Action, now: Millis) {
        let Some(peek) = self.peek else { return };
        match action {
            // With the cart out, about to go in: A plays on from the save,
            // X starts afresh, Y blows on it and B puts it back.
            Action::GbaDown(Btn::A) if peek.taken_out(now) => self.play_held = Some(now),
            Action::GbaDown(Btn::X) if peek.taken_out(now) => self.start_fresh(),
            Action::GbaDown(Btn::Y) if peek.taken_out(now) => self.blow_on_cart(now),
            Action::GbaDown(Btn::A) => self.peek = Some(peek.open(now)),
            Action::GbaUp(Btn::A) => {
                if self.play_held.take().is_some() {
                    self.insert();
                }
            }
            // A loose cart from the store goes back on the shelf.
            Action::GbaDown(Btn::B) if self.store_carts() => self.close_peek(now, None),
            Action::GbaDown(Btn::B) => {
                self.play_held = None;
                self.peek = Some(peek.flip(now))
            }
            Action::ShelfLeft | Action::GbaDown(Btn::Left) => self.close_peek(now, Some(-1)),
            Action::ShelfRight | Action::GbaDown(Btn::Right) => self.close_peek(now, Some(1)),
            Action::GbaDown(Btn::Up) => {
                self.drop_peek();
                self.switch_shelf(-1)
            }
            Action::GbaDown(Btn::Down) => {
                self.drop_peek();
                self.switch_shelf(1)
            }
            Action::GbaDown(Btn::Start) => {
                self.play_held = None;
                self.help = true
            }
            Action::QuickMenu => {
                self.drop_peek();
                self.open_quick_menu()
            }
            Action::GbaDown(Btn::L2) => self.step_display(now),
            Action::GbaDown(Btn::R2) => {
                self.drop_peek();
                self.set_sort(self.state.sort.next(), now, true)
            }
            Action::GbaDown(Btn::Y) => self.toggle_favourite(now),
            _ => {}
        }
    }

    pub fn blow(&self) -> Option<shelf_ui::Blow> {
        self.blow.filter(|b| b.active(self.now()))
    }

    fn settle_blow(&mut self, now: Millis) {
        if self.blow.is_some_and(|b| !b.active(now)) {
            self.blow = None;
            if let Some(rom) = self.blow_rom.take() {
                self.mark_played(&rom);
            }
        }
    }

    pub fn powering_off(&self) -> bool {
        self.powering_off
    }

    pub fn ready_to_power_off(&self) -> bool {
        self.powering_off && self.now() >= self.act_at
    }

    pub fn ready_to_restart(&self) -> bool {
        self.restarting && self.now() >= self.act_at
    }

    /// Exit was picked on the menu: the pak loop drops back to NextUI.
    pub fn exiting(&self) -> bool {
        self.exiting
    }

    pub fn shutting_down(&self) -> bool {
        self.powering_off || self.restarting
    }

    pub fn restarting(&self) -> bool {
        self.restarting
    }

    pub fn restart(&mut self) {
        self.settle_saves();
        if let Some(power) = &mut self.power {
            power.restart();
        }
    }

    pub fn selected_stem(&self) -> Option<&str> {
        self.shelf()
            .carts
            .get(self.shelf().index)
            .map(|c| c.stem.as_str())
    }

    pub fn battery(&self) -> Option<Battery> {
        self.battery
    }

    pub fn poweroff(&mut self) {
        self.settle_saves();
        if let Some(power) = &mut self.power {
            power.poweroff();
        }
    }

    pub fn apply(&mut self, action: Action) {
        self.idle_ms = 0.0;
        if action == Action::PowerHold {
            return self.raise_power_menu();
        }
        if action == Action::PowerTap {
            // A game on its way into the slot is not to be interrupted.
            self.sleep_wanted = !matches!(self.phase, Phase::Inserting { .. });
            return;
        }
        if let Phase::SetClock {
            picker, from_menu, ..
        } = &mut self.phase
        {
            match action {
                Action::GbaDown(Btn::Left) | Action::ShelfLeft => picker.left(),
                Action::GbaDown(Btn::Right) | Action::ShelfRight => picker.right(),
                Action::GbaDown(Btn::Up) => picker.up(),
                Action::GbaDown(Btn::Down) => picker.down(),
                Action::GbaDown(Btn::A) => self.confirm_clock(),
                Action::GbaDown(Btn::B) if *from_menu => {
                    self.phase = Phase::QuickMenu {
                        row: QuickRow::DateTime,
                    }
                }
                _ => {}
            }
            return;
        }
        if self.adjust(action) {
            return;
        }
        match action {
            Action::GbaUp(Btn::Left) => self.shelf_mut().release_left(),
            Action::GbaUp(Btn::Right) => self.shelf_mut().release_right(),
            _ => {}
        }
        let now = self.now();
        // A button with the info card up puts it away, and then does what
        // it does on the box's back: B turns the box round, as it did before.
        if matches!(
            action,
            Action::GbaDown(_) | Action::QuickMenu | Action::ShelfLeft | Action::ShelfRight
        ) && self.info.is_some_and(|i| !i.away)
        {
            self.close_info(now);
        }
        match self.phase {
            Phase::Shelf => match action {
                // Any button tears the receipt off and does nothing else.
                Action::GbaDown(_) | Action::QuickMenu if self.slip.is_some() => self.tear_slip(now),
                _ if self.slip.is_some() => {}
                // Any button closes the guide and does nothing else.
                Action::GbaDown(_) | Action::QuickMenu if self.help => self.help = false,
                _ if self.help => {}
                // The shelf is spinning: wait for it to stop.
                _ if self.spin.is_some() => {}
                // B on a box's back brings up its info card, as X does
                // anywhere but with the cart out.
                Action::GbaDown(Btn::B) if self.info_ready() => self.open_info(now),
                Action::GbaDown(Btn::X) if !self.peek.is_some_and(|p| p.taken_out(now)) => {
                    self.open_info(now)
                }
                _ if self.peek.is_some() => self.peek_input(action, now),
                // A loose cart in the store held up as its box: A puts the
                // box away and takes the cart out, as A on the cart does.
                Action::GbaDown(Btn::A) if self.store_carts() && self.box_view.is_some() => {
                    self.box_view = None;
                    self.open_peek(now)
                }
                Action::GbaDown(Btn::A) if self.boxes() || self.store() => self.open_peek(now),
                Action::GbaDown(Btn::B) if self.boxes() || self.store_boxes() => self.flip_box(now),
                Action::GbaDown(Btn::A) if self.box_view.is_some() => {
                    self.box_view = None;
                    self.play_held = Some(now)
                }
                Action::GbaDown(Btn::B) => self.turn_box(now),
                Action::GbaDown(_) | Action::QuickMenu if self.box_open() => {
                    self.box_view = self.box_view.map(|b| b.closed(now))
                }
                Action::GbaDown(Btn::L2) => self.step_display(now),
                Action::GbaDown(Btn::R2) => self.set_sort(self.state.sort.next(), now, true),
                Action::GbaDown(Btn::Start) => {
                    self.play_held = None;
                    self.help = true
                }
                Action::GbaDown(Btn::Select) if self.box_view.is_none() => {
                    self.spin_for_a_game(now)
                }
                Action::GbaDown(Btn::Y) => self.toggle_favourite(now),
                Action::GbaDown(Btn::Up) => self.switch_shelf(-1),
                Action::GbaDown(Btn::Down) => self.switch_shelf(1),
                Action::ShelfLeft | Action::GbaDown(Btn::Left) => self.shelf_mut().hold_left(now),
                Action::ShelfRight | Action::GbaDown(Btn::Right) => {
                    self.shelf_mut().hold_right(now)
                }
                Action::QuickMenu => self.open_quick_menu(),
                Action::GbaDown(Btn::A) => self.play_held = Some(now),
                Action::GbaUp(Btn::A) => {
                    if self.play_held.take().is_some() {
                        self.insert();
                    }
                }
                Action::GbaDown(Btn::L1) => self.jump_letter(-1),
                Action::GbaDown(Btn::R1) => self.jump_letter(1),
                _ => {}
            },
            Phase::About if action == Action::GbaDown(Btn::A) => self.about_back = !self.about_back,
            Phase::About if action == Action::GbaDown(Btn::B) || action == Action::QuickMenu => {
                self.phase = Phase::QuickMenu {
                    row: QuickRow::About,
                }
            }
            Phase::Receipts { at } => self.spike_input(at, action),
            Phase::Artwork => self.artwork_input(action),
            Phase::Search { .. } => self.search_input(action),
            Phase::QuickMenu { row } => self.quick_menu_input(row, action),
            _ => {}
        }
    }

    /// The games on the console shelves, for Get Artwork.
    fn artwork_games(&self) -> Vec<shelf_fetch::Game> {
        let mut games: Vec<shelf_fetch::Game> = self
            .shelves
            .iter()
            .filter(|(key, _)| !key.is_list())
            .flat_map(|(_, shelf)| shelf.carts.iter().map(|c| (c.platform, c.stem.clone())))
            .collect();
        games.sort_by(|a, b| (a.0 as u8, &a.1).cmp(&(b.0 as u8, &b.1)));
        games.dedup();
        games
    }

    /// Where Get Artwork fetches from: LaunchBox, unless a test says
    /// otherwise.
    pub fn set_artwork_net(&mut self, net: Arc<dyn shelf_fetch::Net>) {
        self.artwork_net = Some(net);
    }

    fn open_artwork(&mut self) {
        let games = self.artwork_games();
        let need = match &self.root {
            Some(root) => {
                let (n, bytes) = shelf_fetch::estimate(root, &games);
                (games.len(), n, bytes, shelf_fetch::free_space(root))
            }
            None => (games.len(), 0, 0, None),
        };
        self.artwork_need = need;
        self.artwork_gaps = match &self.root {
            Some(root) => shelf_fetch::missing_by_console(root, &games),
            None => Vec::new(),
        };
        self.phase = Phase::Artwork;
    }

    fn start_artwork(&mut self) {
        let Some(root) = self.root.clone() else {
            return self.refuse();
        };
        let net = self
            .artwork_net
            .clone()
            .unwrap_or_else(|| Arc::new(shelf_fetch::Web::default()));
        // Each game's small copies are cut as its pictures arrive, so the
        // shelf has them ready rather than cutting them as it goes past.
        let carts: HashMap<shelf_fetch::Game, Cart> = self
            .carts()
            .map(|c| ((c.platform, c.stem.clone()), c.clone()))
            .collect();
        let at = root.clone();
        let saved: shelf_fetch::Saved = Arc::new(move |game: &shelf_fetch::Game| {
            if let Some(cart) = carts.get(game) {
                crate::label_cache::make_copies(&at, cart);
            }
        });
        self.artwork = Some(shelf_fetch::Job::start_with(
            root,
            self.artwork_games(),
            net,
            shelf_fetch::PAUSE,
            saved,
        ));
    }

    /// Whether Get Artwork is running, or its screen has words not drawn
    /// yet: frames keep coming until both settle.
    pub fn artwork_running(&self) -> bool {
        let words_due = || self.artwork_faces_text() != self.screen_text().as_ref();
        match self.phase {
            Phase::Artwork => self.artwork.as_ref().is_some_and(|j| !j.finished()) || words_due(),
            Phase::Search { .. } => words_due(),
            _ => false,
        }
    }

    /// The words of the screen drawn like Get Artwork's: that, or Search.
    pub fn screen_text(&self) -> Option<shelf_ui::ArtworkText> {
        self.artwork_text().or_else(|| self.search_text())
    }

    /// The games whose names hold what has been spelt, those starting with
    /// it first, then by name.
    fn search_matches(&self, spelt: &str) -> Vec<&Cart> {
        let want = spelt.trim().to_ascii_lowercase();
        let mut found: Vec<(bool, String, &Cart)> = self
            .carts()
            .filter_map(|c| {
                let name = shelf_ui::game_name(&c.stem).to_ascii_lowercase();
                name.find(&want).map(|at| (at != 0, name, c))
            })
            .collect();
        found.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        found.into_iter().map(|(_, _, c)| c).collect()
    }

    fn spelt(query: &str, letter: usize) -> String {
        let mut spelt = query.to_string();
        spelt.push(SEARCH_LETTERS[letter] as char);
        spelt
    }

    fn search_text(&self) -> Option<shelf_ui::ArtworkText> {
        let Phase::Search { query, letter, pick } = &self.phase else {
            return None;
        };
        let spelt = Self::spelt(query, *letter);
        let matches = self.search_matches(&spelt);
        let shown = match SEARCH_LETTERS[*letter] {
            b' ' => format!("{query}_"),
            c => format!("{query}[{}]", c as char),
        };
        let mut lines = vec![shown];
        let start = pick.saturating_sub(SEARCH_SHOWN - 1);
        for (n, c) in matches.iter().enumerate().skip(start).take(SEARCH_SHOWN) {
            let name = shelf_ui::game_name(&c.stem);
            let console = c.platform.name();
            lines.push(match n == *pick {
                true => format!("> {name} \u{b7} {console} <"),
                false => format!("{name} \u{b7} {console}"),
            });
        }
        lines.push(match matches.len() {
            0 => "No games match".to_string(),
            1 => "1 game".to_string(),
            n => format!("{n} games"),
        });
        lines.push("Up and Down pick a letter, Right adds another".to_string());
        Some(shelf_ui::ArtworkText {
            title: "Search",
            lines,
            hints: vec![("B", "Delete"), ("R", "Next"), ("A", "Go")],
        })
    }

    fn open_search(&mut self) {
        self.phase = Phase::Search {
            query: String::new(),
            letter: 0,
            pick: 0,
        };
    }

    fn search_input(&mut self, action: Action) {
        let Phase::Search { query, letter, pick } = &self.phase else {
            return;
        };
        let (mut query, mut letter, mut pick) = (query.clone(), *letter, *pick);
        let n = SEARCH_LETTERS.len();
        match action {
            Action::GbaDown(Btn::Up) => letter = (letter + 1) % n,
            Action::GbaDown(Btn::Down) => letter = (letter + n - 1) % n,
            Action::GbaDown(Btn::Right) if letter != 0 => {
                query.push(SEARCH_LETTERS[letter] as char);
                letter = 0;
            }
            Action::GbaDown(Btn::B | Btn::Left) => {
                if letter != 0 {
                    letter = 0;
                } else if let Some(c) = query.pop() {
                    letter = SEARCH_LETTERS.iter().position(|l| *l as char == c).unwrap_or(0);
                } else {
                    return self.close_quick_menu();
                }
            }
            Action::GbaDown(Btn::R1) => pick += 1,
            Action::GbaDown(Btn::L1) => pick = pick.saturating_sub(1),
            Action::GbaDown(Btn::A) => {
                let spelt = Self::spelt(&query, letter);
                let rom = self.search_matches(&spelt).get(pick).map(|c| c.rom.clone());
                if let Some(rom) = rom {
                    self.close_quick_menu();
                    self.seat_rom(&rom);
                }
                return;
            }
            Action::GbaDown(Btn::Start) | Action::QuickMenu => return self.close_quick_menu(),
            _ => return,
        }
        let found = self.search_matches(&Self::spelt(&query, letter)).len();
        pick = pick.min(found.saturating_sub(1));
        self.phase = Phase::Search {
            query,
            letter,
            pick,
        };
    }

    /// Bring the shelf round to `rom`'s console with it in the middle.
    fn seat_rom(&mut self, rom: &Path) {
        let found = self.shelves.iter().enumerate().find_map(|(at, (key, shelf))| {
            let i = shelf.carts.iter().position(|c| c.rom == rom)?;
            (!key.is_list()).then_some((at, i))
        });
        let Some((at, i)) = found else {
            return;
        };
        if at != self.shelf_at {
            self.switch_shelf_to(at);
        }
        self.shelf_mut().select(i);
        self.name_pending = true;
    }

    fn artwork_input(&mut self, action: Action) {
        if self.restarting {
            return;
        }
        let status = self.artwork.as_ref().map(|j| j.status().stage);
        let back = matches!(action, Action::GbaDown(Btn::B) | Action::QuickMenu);
        match status {
            // Running: B stops it, at the next picture.
            Some(shelf_fetch::Stage::Reading { .. } | shelf_fetch::Stage::Fetching { .. }) => {
                if back {
                    if let Some(job) = &self.artwork {
                        job.stop();
                    }
                }
            }
            // Stopped part way: A carries on where it stopped.
            Some(shelf_fetch::Stage::Finished(s)) if s.stopped.is_some() => match action {
                Action::GbaDown(Btn::A) => self.start_artwork(),
                _ if back => self.leave_artwork(),
                _ => {}
            },
            // Done: A restarts Shelf, which reads the new pictures.
            Some(shelf_fetch::Stage::Finished(s)) if s.pictures + s.details > 0 => match action {
                Action::GbaDown(Btn::A) => {
                    self.restarting = true;
                    self.act_at = self.now() + SHUTDOWN_SHOW_MS;
                }
                _ if back => self.leave_artwork(),
                _ => {}
            },
            _ => match action {
                Action::GbaDown(Btn::A) => self.start_artwork(),
                _ if back => self.leave_artwork(),
                _ => {}
            },
        }
    }

    fn leave_artwork(&mut self) {
        self.artwork = None;
        self.phase = Phase::QuickMenu {
            row: QuickRow::GetArtwork,
        };
    }

    /// How far the run has got, for the bar, while it runs.
    pub fn artwork_progress(&self) -> Option<f32> {
        match self.artwork.as_ref()?.status().stage {
            shelf_fetch::Stage::Reading { read, of } if of > 0 => Some(read as f32 / of as f32),
            shelf_fetch::Stage::Reading { .. } => Some(0.0),
            shelf_fetch::Stage::Fetching { done, of } if of > 0 => Some(done as f32 / of as f32),
            _ => None,
        }
    }

    /// What the Get Artwork screen says now.
    pub fn artwork_text(&self) -> Option<shelf_ui::ArtworkText> {
        if !matches!(self.phase, Phase::Artwork) {
            return None;
        }
        let mb = |b: u64| (b + (1 << 20) - 1) >> 20;
        let mut lines = Vec::new();
        let status = self.artwork.as_ref().map(|j| j.status());
        let hints = match status.as_ref().map(|s| &s.stage) {
            None => {
                let (games, n, bytes, free) = self.artwork_need;
                lines.push(format!(
                    "Boxes, carts, screenshots and details for {games} games"
                ));
                lines.push(match n {
                    0 => "Every picture is on the card already".to_string(),
                    _ => format!("{n} pictures missing, up to {} MB", mb(bytes)),
                });
                // Which consoles the gaps are on: fronts / backs / carts.
                let gaps: Vec<String> = self
                    .artwork_gaps
                    .iter()
                    .filter(|(_, g)| g.iter().any(|n| *n > 0))
                    .map(|(p, [f, b, c])| {
                        let short = match p {
                            Platform::Nes => "NES",
                            Platform::Snes => "SNES",
                            p => p.dir_name(),
                        };
                        format!("{short} {f}/{b}/{c}")
                    })
                    .collect();
                if !gaps.is_empty() {
                    lines.push("Missing fronts / backs / carts:".to_string());
                    for row in gaps.chunks(4) {
                        lines.push(row.join(" \u{b7} "));
                    }
                }
                if let Some(free) = free {
                    lines.push(format!("{} MB free on the SD card", free >> 20));
                }
                lines.push("Needs Wi-Fi".to_string());
                vec![("A", "Start"), ("B", "Back")]
            }
            Some(shelf_fetch::Stage::Reading { read, of }) => {
                lines.push("Reading LaunchBox's list of games".to_string());
                lines.push(format!("{} of {} MB", read >> 20, mb(*of)));
                vec![("B", "Stop")]
            }
            Some(shelf_fetch::Stage::Fetching { done, of }) => {
                lines.push(format!("Fetching picture {} of {of}", done + 1));
                lines.push(
                    status
                        .as_ref()
                        .and_then(|s| s.game.clone())
                        .unwrap_or_default(),
                );
                vec![("B", "Stop")]
            }
            Some(shelf_fetch::Stage::Finished(s)) => {
                lines.push(match (s.pictures, s.details) {
                    (0, 0) => "Nothing new to add".to_string(),
                    (p, 0) => format!("{p} pictures added"),
                    (0, d) => format!("Details added for {d} games"),
                    (p, d) => format!("{p} pictures and details for {d} games added"),
                });
                if let Some(why) = &s.stopped {
                    lines.push(why.clone());
                }
                if s.unknown > 0 {
                    lines.push(format!("{} games are not in LaunchBox", s.unknown));
                }
                if s.failed > 0 {
                    lines.push(format!("{} pictures could not be fetched", s.failed));
                }
                match s.pictures + s.details {
                    _ if s.stopped.is_some() => vec![("A", "Continue"), ("B", "Back")],
                    0 => vec![("A", "Try again"), ("B", "Back")],
                    _ => {
                        lines.push("Restart Shelf to see them".to_string());
                        vec![("A", "Restart"), ("B", "Back")]
                    }
                }
            }
            Some(shelf_fetch::Stage::Failed(e)) => {
                lines.push("Could not get the artwork".to_string());
                lines.push(e.chars().take(60).collect());
                lines.push("Check the Wi-Fi and try again".to_string());
                vec![("A", "Try again"), ("B", "Back")]
            }
        };
        lines.push(shelf_ui::ARTWORK_CREDIT.to_string());
        Some(shelf_ui::ArtworkText {
            title: shelf_ui::ARTWORK_TITLE,
            lines,
            hints,
        })
    }

    pub fn set_artwork_faces(&mut self, faces: shelf_ui::ArtworkFaces) {
        self.artwork_faces = Some(faces);
    }

    pub fn take_artwork_faces(&mut self) -> Option<shelf_ui::ArtworkFaces> {
        self.artwork_faces.take()
    }

    pub fn artwork_faces_text(&self) -> Option<&shelf_ui::ArtworkText> {
        self.artwork_faces.as_ref().map(|f| &f.text)
    }

    fn open_quick_menu(&mut self) {
        self.help = false;
        self.phase = Phase::QuickMenu {
            row: QuickRow::TOP[0],
        };
    }

    /// Holding the power button: the menu, with Power Off selected. A
    /// screen that is not the shelf, such as the first boot's clock, is
    /// kept to come back to.
    fn raise_power_menu(&mut self) {
        if matches!(self.phase, Phase::QuickMenu { .. }) {
            return;
        }
        self.help = false;
        let under = std::mem::replace(
            &mut self.phase,
            Phase::QuickMenu {
                row: QuickRow::PowerOff,
            },
        );
        if !matches!(under, Phase::Shelf | Phase::About | Phase::Receipts { .. }) {
            self.parked = Some(under);
        }
    }

    fn close_quick_menu(&mut self) {
        self.phase = self.parked.take().unwrap_or(Phase::Shelf);
    }

    fn quick_menu_input(&mut self, row: QuickRow, action: Action) {
        let row = match action {
            Action::GbaDown(Btn::Up) => row.up(),
            Action::GbaDown(Btn::Down) => row.down(),
            Action::GbaDown(Btn::Left) => return self.change_setting(row, false),
            Action::GbaDown(Btn::Right) => return self.change_setting(row, true),
            Action::GbaDown(Btn::A) => return self.open_quick_row(row),
            Action::GbaDown(Btn::B) if row.in_options() => QuickRow::Options,
            Action::GbaDown(Btn::B | Btn::Start) | Action::QuickMenu => {
                return self.close_quick_menu();
            }
            _ => return,
        };
        self.phase = Phase::QuickMenu { row };
    }

    fn open_quick_row(&mut self, row: QuickRow) {
        match row {
            QuickRow::Options => {
                self.phase = Phase::QuickMenu {
                    row: QuickRow::SETTINGS[0],
                }
            }
            QuickRow::DateTime => {
                self.phase = clock_screen(self.utc_secs(), self.state.utc_offset_min, true);
            }
            QuickRow::About => {
                self.about_back = false;
                self.phase = Phase::About
            }
            QuickRow::Receipts => {
                self.receipts = self
                    .root
                    .as_deref()
                    .map(shelf_store::read_receipts)
                    .unwrap_or_default();
                self.phase = Phase::Receipts { at: 0 }
            }
            QuickRow::GetArtwork => self.open_artwork(),
            QuickRow::Hide => self.toggle_hidden(),
            QuickRow::Search => self.open_search(),
            QuickRow::Guide => {
                self.close_quick_menu();
                self.play_held = None;
                self.help = true;
            }
            QuickRow::Restart => {
                self.close_quick_menu();
                self.restarting = true;
                self.act_at = self.now() + SHUTDOWN_SHOW_MS;
            }
            QuickRow::PowerOff => {
                self.close_quick_menu();
                self.begin_power_off();
            }
            QuickRow::Exit => {
                self.close_quick_menu();
                self.exiting = true;
            }
            QuickRow::Display
            | QuickRow::View
            | QuickRow::Sort
            | QuickRow::Crt
            | QuickRow::Dusty
            | QuickRow::Resume
            | QuickRow::ShowHidden
            | QuickRow::PrintReceipts => {}
        }
    }

    fn change_setting(&mut self, row: QuickRow, right: bool) {
        let now = self.now();
        match row {
            QuickRow::Display => return self.set_theme(!self.store(), now),
            QuickRow::View => return self.set_view(!self.view_carts(), now, false),
            QuickRow::Sort => {
                let key = match right {
                    true => self.state.sort.next(),
                    false => self.state.sort.prev(),
                };
                return self.set_sort(key, now, false);
            }
            QuickRow::Crt => self.state.crt = !self.state.crt,
            QuickRow::Dusty => {
                self.state.dusty = !self.state.dusty;
                self.refresh_dust();
            }
            QuickRow::Resume => self.state.resume = !self.state.resume,
            QuickRow::PrintReceipts => self.state.receipts = !self.state.receipts,
            QuickRow::ShowHidden => {
                self.state.show_hidden = !self.state.show_hidden;
                self.apply_hidden();
            }
            _ => return,
        }
        self.persist();
    }

    pub fn apply_at(&mut self, action: Action, now: Millis) {
        self.clock = now as f64;
        self.apply(action);
    }

    fn adjust(&mut self, action: Action) -> bool {
        if action == Action::MuteToggle {
            self.mute_toggle();
            return true;
        }
        let s = &self.state;
        let (kind, value) = match action {
            Action::BrightnessUp => (HudKind::Brightness, up(s.brightness, 1, BRIGHTNESS_MAX)),
            Action::BrightnessDown => (HudKind::Brightness, s.brightness.saturating_sub(1)),
            Action::VolumeUp => (HudKind::Volume, up(self.level(), VOLUME_STEP, VOLUME_MAX)),
            Action::VolumeDown => (HudKind::Volume, self.level().saturating_sub(VOLUME_STEP)),
            _ => return false,
        };
        if kind == HudKind::Volume {
            self.remember_volume();
        }
        let level = match kind {
            HudKind::Brightness => &mut self.state.brightness,
            HudKind::Volume => &mut self.state.volume,
        };
        let moved = *level != value;
        *level = value;
        let unmuted = kind == HudKind::Volume && std::mem::take(&mut self.state.muted);
        let (shown, now) = (self.hud_value(kind, value), self.now());
        self.hud.show(kind, shown, self.muted(), now);
        if let (HudKind::Brightness, Some(power)) = (kind, &mut self.power) {
            power.set_backlight(value);
        }
        if moved || unmuted {
            self.persist_soon();
        }
        true
    }

    fn remember_volume(&mut self) {
        if self.vol_before.len() == 2 {
            self.vol_before.remove(0);
        }
        self.vol_before
            .push((self.level(), self.muted(), self.now()));
    }

    fn mute_toggle(&mut self) {
        let now = self.now();
        if let Some((volume, muted, _)) = self
            .vol_before
            .iter()
            .find(|(_, _, at)| now.saturating_sub(*at) <= MUTE_CHORD_MS)
            .copied()
        {
            self.state.volume = volume;
            self.state.muted = muted;
        }
        self.vol_before.clear();
        let muted = !self.muted();
        self.state.muted = muted;
        self.hud
            .show(HudKind::Volume, self.output_volume(), muted, now);
        self.persist_soon();
    }

    fn hud_value(&self, kind: HudKind, value: u8) -> u8 {
        match kind {
            HudKind::Volume => self.output_volume(),
            _ => value,
        }
    }

    pub fn brightness(&self) -> u8 {
        self.state.brightness
    }

    pub fn volume(&self) -> u8 {
        self.level()
    }

    fn level(&self) -> u8 {
        self.state.volume
    }

    pub fn muted(&self) -> bool {
        self.state.muted
    }

    pub fn output_volume(&self) -> u8 {
        if self.muted() {
            0
        } else {
            self.level()
        }
    }

    pub fn hud_icon(&self) -> Icon {
        self.hud.glyph()
    }

    pub fn now(&self) -> Millis {
        self.clock as Millis
    }

    /// Whether something is under way that the next update must carry on,
    /// even if nothing shows yet: a direction or A held down, waiting to
    /// repeat or to turn into a blow, or the shake of a refused game. The
    /// rest of what moves shows in the frame itself.
    pub fn animating(&self) -> bool {
        self.artwork_running()
            || self.play_held.is_some()
            || self.shelf().held()
            || self.refusal.is_some_and(|r| r.active(self.now()))
            || self.details_rx.is_some()
            || self.slip.is_some()
            || self.peek_step.is_some()
            // The info card coming up or going away.
            || self
                .info
                .is_some_and(|i| i.away || i.shown(self.now()) < 1.0)
            // A box opening or shutting: carry on until it is settled, so a
            // finished peek is never left for the next press to find.
            || self
                .peek
                .is_some_and(|p| !p.at_rest(self.now()) || p.put_away(self.now()))
            || (self.store() && self.now().saturating_sub(self.sign_at) <= shelf_ui::SIGN_IN_MS)
    }

    pub fn update(&mut self, dt: f32) {
        self.clock += dt as f64 * 1000.0;
        self.count_idle(dt);
        self.poll_details();
        self.poll_picks();
        self.keep_sign();
        self.keep_shelf_named();
        if self.name_pending && self.shelf_platform.face.is_some() {
            self.name_pending = false;
            self.shelf_named = Some(self.now());
        }
        self.timers();
        self.note_saved();
        let now = self.now();
        self.settle_blow(now);
        self.settle_box(now);
        self.settle_peek(now);
        self.settle_info(now);
        self.turn_spin(now);
        self.settle_slip(now);
        if !self.on_shelf() {
            self.shelf_mut().release_hold();
        }
        let mut touched = false;
        let at = self.shelf_at;
        match &mut self.phase {
            Phase::Shelf => {
                let shelf = &mut self.shelves[at].1;
                shelf.tick(now);
                shelf.update(dt);
            }
            Phase::Inserting { t, .. } => {
                let was = *t;
                *t += dt;
                let at = SEATED_AT - Sfx::Insert.lead();
                touched = was < at && *t >= at;
            }
            _ => {}
        }
        if touched {
            self.sfx = Some(Sfx::Insert);
        }
        self.hand_off();
    }

    /// Whether games should continue from NextUI's last save state.
    pub fn resume_saves(&self) -> bool {
        self.state.resume && !self.fresh
    }

    /// Whether B would bring up the info card: the box is turning over to
    /// its back or resting on it, so that a quick second B still gets the
    /// card, and its game has details to put on one.
    fn info_ready(&self) -> bool {
        self.info.is_none()
            && self.peek.or(self.box_view).is_some_and(|v| v.bound_for_back())
            && self.info_key().is_some()
    }

    /// Bring up the info card for the game in the middle, if anything is
    /// known of it.
    fn open_info(&mut self, now: Millis) {
        if self.info.is_some() {
            return;
        }
        let Some(rom) = self.selected_cart().map(|c| c.rom.clone()) else {
            return self.refuse();
        };
        self.load_details_of(&rom);
        if self.details_of(&rom).is_none() {
            return self.refuse();
        }
        self.play_held = None;
        self.info = Some(InfoCard {
            since: now,
            away: false,
        })
    }

    fn close_info(&mut self, now: Millis) {
        if let Some(info) = self.info.filter(|i| !i.away) {
            self.info = Some(info.closed(now));
        }
    }

    /// Let go of the info card once it has gone, or its box has.
    fn settle_info(&mut self, now: Millis) {
        let Some(info) = self.info else { return };
        let gone = info.away && info.shown(now) <= 0.0;
        if gone || !self.on_shelf() {
            self.info = None;
        }
    }

    /// What the info card for the game in hand is made for, while its
    /// box is on its back or the card is up.
    pub fn info_key(&self) -> Option<InfoKey> {
        if !self.on_shelf() || !(self.info.is_some() || self.box_back_showing(self.now())) {
            return None;
        }
        let cart = self.selected_cart()?;
        self.details_of(&cart.rom)?;
        let offset = i64::from(self.state.utc_offset_min) * 60;
        let rental = self
            .rental_of(&cart.rom)
            .and_then(|r| shelf_ui::RentalSticker::new(r.times, r.last + offset))
            .map(|r| r.worded(self.plain()));
        let shots = match &self.root {
            Some(root) => ["title", "play"]
                .into_iter()
                .filter(|kind| shot_of(root, cart, kind).is_some())
                .count(),
            None => 0,
        };
        Some(InfoKey {
            rom: cart.rom.clone(),
            rental,
            shots,
        })
    }

    /// What the info card for `key` is made from: the game's details, its
    /// console, its ROM's name and its screenshots on the card.
    pub fn info_source(&self) -> Option<(Details, Platform, String, Vec<PathBuf>)> {
        let cart = self.selected_cart()?;
        let details = self.details_of(&cart.rom)?.clone();
        let shots = match &self.root {
            Some(root) => ["title", "play"]
                .into_iter()
                .filter_map(|kind| shot_of(root, cart, kind))
                .collect(),
            None => Vec::new(),
        };
        Some((details, cart.platform, cart.stem.clone(), shots))
    }

    pub fn set_info_face(&mut self, key: InfoKey, tex: TexId) {
        self.info_face = Some((key, tex));
    }

    /// The info card over everything, while it is up or on its way.
    fn draw_info_card(&self, out: &mut Vec<Draw>) {
        let Some(info) = self.info else { return };
        let key = self.info_key();
        if let Some((_, tex)) = self
            .info_face
            .as_ref()
            .filter(|(k, _)| Some(k) == key.as_ref())
        {
            shelf_ui::draw_info(*tex, info.shown(self.now()), out);
        }
    }

    /// Whether a box shows its back, turned over on the shelf or held up.
    fn box_back_showing(&self, now: Millis) -> bool {
        self.peek.is_some_and(|p| p.on_back(now)) || self.box_view.is_some_and(|b| b.on_back(now))
    }

    /// X on the back of a saved game's box: play it from its start, as
    /// NextUI's own start button does, leaving the save states alone.
    fn start_fresh(&mut self) {
        self.drop_peek();
        self.box_view = None;
        self.play_held = None;
        self.insert();
        self.fresh = matches!(self.phase, Phase::Inserting { .. });
    }

    pub fn handed_off(&self) -> Option<&Path> {
        self.handed.as_deref()
    }

    fn hand_off(&mut self) {
        if self.handed.is_some() {
            return;
        }
        let Phase::Inserting { cart, t } = &self.phase else {
            return;
        };
        if *t < INSERT_S {
            return;
        }
        let cart = cart.clone();
        let Some(rom) = self.seated_cart().map(|c| c.rom.clone()) else {
            return;
        };
        self.note_place();
        self.record_cart(Some(cart));
        if let Some(root) = &self.root {
            let wall = self.utc_secs();
            let boot = shelf_store::boot_id();
            let at = monotonic_secs();
            if let Err(e) = shelf_store::start_playing(root, &rom, at, wall, boot.as_deref()) {
                eprintln!("shelf: playing: {e}");
            }
        }
        self.handed = Some(rom);
    }

    /// 1.0 when the shelf has been put away for a menu, 0.0 while the shelf
    /// or a cart going into its slot is showing.
    pub fn frame_lift(&self) -> f32 {
        let docked = matches!(self.phase, Phase::Shelf | Phase::Inserting { .. });
        if docked {
            0.0
        } else {
            1.0
        }
    }

    pub fn frame_split(&self) -> Option<f32> {
        match &self.phase {
            Phase::QuickMenu { .. } => Some(shelf_ui::QUICK_SPLIT),
            _ => None,
        }
    }

    pub fn tick_ms(&mut self, now: Millis) {
        self.clock = self.clock.max(now as f64);
        self.timers();
    }

    fn timers(&mut self) {
        if self.now() >= self.card_check_at {
            self.card_check_at = self.now() + CARD_CHECK_MS;
            self.card_read_only = self.power.as_ref().is_some_and(|p| p.card_read_only());
        }
        let failing = self.card_failing.load(Ordering::Relaxed) || self.card_read_only;
        if failing != self.hud.alarmed() {
            eprintln!(
                "shelf: card: alarm {} (read-only {})",
                if failing { "on" } else { "off" },
                self.card_read_only
            );
        }
        self.hud.set_alarm(failing, self.now());
        if self.now() >= self.battery_at {
            self.battery_at = self.now() + BATTERY_POLL_MS;
            self.battery = self.power.as_ref().and_then(|p| p.battery());
            if let Some(b) = self.battery {
                self.on_battery(b);
            }
        }
        if self.now() >= self.charge_at {
            self.charge_at = self.now() + CHARGE_POLL_MS;
            if let (Some(power), Some(b)) = (self.power.as_ref(), self.battery.as_mut()) {
                b.charge = power.charge();
            }
        }
    }

    /// Keep the game being played, and where every shelf was left, for
    /// when Shelf comes back after it.
    fn record_cart(&mut self, cart: Option<String>) {
        let platform = self.seated_cart().map(|c| c.platform);
        self.state.cart = cart;
        self.state.cart_platform = platform;
        self.persist();
    }

    fn persist(&self) {
        let Some(root) = &self.root else {
            return;
        };
        self.state_writer.flush();
        let written = write_shelf_state(root, &self.state);
        if let Err(e) = &written {
            eprintln!("shelf: shelf.state: {e}");
        }
        self.card_failing.store(written.is_err(), Ordering::Relaxed);
    }

    fn persist_soon(&self) {
        if let Some(root) = &self.root {
            self.state_writer.save(root, &self.state);
        }
    }

    pub fn flush_state(&self) {
        self.state_writer.flush();
    }

    pub fn refuse(&mut self) {
        self.refusal = Some(Refusal::started(self.now()));
    }

    pub fn refusal_active(&self, now: Millis) -> bool {
        self.refusal.is_some_and(|r| r.active(now))
    }

    pub fn seat(&self) -> f32 {
        match &self.phase {
            Phase::Shelf => 0.0,
            Phase::Inserting { t, .. } => (t / SEATED_AT).clamp(0.0, 1.0),
            _ => 1.0,
        }
    }

    pub fn draw(&self, out: &mut Vec<Draw>) {
        self.draw_screen(out);
        if let (Phase::Shelf, Some(slip)) = (&self.phase, self.slip) {
            let now = self.now();
            let at = slip.at.unwrap_or(now);
            let t = now.saturating_sub(at) as f32 / shelf_ui::SLIP_PRINT_MS as f32;
            let tear = slip.torn.map_or(0.0, |torn| {
                now.saturating_sub(torn) as f32 / shelf_ui::SLIP_TEAR_MS as f32
            });
            shelf_ui::draw_receipt_slip(self.receipt_tex(), t, tear, out);
        }
    }

    fn draw_screen(&self, out: &mut Vec<Draw>) {
        if self.shutting_down() {
            out.push(Draw::Rect {
                x: 0.0,
                y: -shelf_ui::band(),
                w: OUT_W as f32,
                h: OUT_H as f32 + shelf_ui::band(),
                colour: [0.0, 0.0, 0.0, 1.0],
            });
            let which = if self.restarting {
                PowerChoice::Restart
            } else {
                PowerChoice::PowerOff
            };
            if let Some((tex, w, h)) = self.shutdown_faces.get(which.index()).copied() {
                out.push(Draw::Tex {
                    x: ((OUT_W - w) / 2) as f32,
                    y: ((OUT_H - h) / 2) as f32,
                    w: w as f32,
                    h: h as f32,
                    tex,
                    alpha: 1.0,
                });
            }
            return;
        }
        match &self.phase {
            Phase::SetClock {
                picker, from_menu, ..
            } => {
                let (line, hint) = match self.clock_faces {
                    Some((line, hint)) => (Some(line), Some(hint)),
                    None => (None, None),
                };
                let back = self
                    .quick_menu_faces
                    .as_ref()
                    .filter(|_| *from_menu)
                    .map(|f| f.legend[0]);
                picker.draw(line, hint, back, out);
                return;
            }
            Phase::QuickMenu { row } => QuickMenu {
                row: *row,
                values: QuickRow::ALL.map(|r| self.quick_value(r)),
                clock: self.quick_clock_faces,
                faces: self.quick_menu_faces.as_ref(),
                unhide: self.selected_hidden(),
            }
            .draw(out),
            Phase::Shelf if self.store() => {
                self.draw_store_scene(out);
                // With a cart in hand the console has come up over the
                // strip, and its plate carries the name instead of the ticket.
                let ticket = self.store_ticket.filter(|_| self.peek.is_none());
                shelf_ui::draw_store_footer(
                    ticket,
                    self.slot_name_alpha() / SLOT_NAME_ALPHA,
                    self.battery,
                    self.battery_percent,
                    self.bolt,
                    Some(self.wall_secs()),
                    self.peek.is_some(),
                    out,
                );
                self.draw_info_card(out);
                if self.help {
                    shelf_ui::draw_help(&self.help_faces, &self.help_lines(), out);
                }
                if self.state.crt {
                    shelf_ui::draw_crt(out);
                }
            }
            Phase::Shelf if self.boxes() => {
                self.draw_boxes_scene(out);
                draw_footer(
                    self.battery,
                    self.battery_percent,
                    self.bolt,
                    self.shelf_clock,
                    out,
                );
                self.draw_info_card(out);
                if self.help {
                    shelf_ui::draw_help(&self.help_faces, &self.help_lines(), out);
                }
                if self.state.crt {
                    shelf_ui::draw_crt(out);
                }
            }
            Phase::Shelf => {
                draw_backdrop(self.wallpaper, out);
                match (self.blow(), self.box_view) {
                    (_, Some(view)) => {
                        let now = self.now();
                        self.shelf()
                            .draw_turned(self.shelf_shake(), view.cart_squash(now), out);
                    }
                    (Some(blow), None) => {
                        self.shelf()
                            .draw_blown(self.shelf_shake(), &blow, self.now(), out);
                    }
                    (None, None) => {
                        self.shelf().draw(self.shelf_shake(), out);
                        // The CONTINUE? note, as on a cart out of its box.
                        self.draw_note_on_cart(out);
                    }
                }
                self.draw_game_name(out);
                self.draw_section_tab(out);
                draw_slot_name(self.shelf_platform, self.slot_name_alpha(), out);
                if let (Some(view), Some(rect)) = (self.box_view, self.shelf().selected_rect()) {
                    let (face, back, details, plain) = self.box_view_faces();
                    view.draw(
                        self.now(),
                        rect,
                        self.shelf().selected_face(),
                        face,
                        back,
                        details,
                        plain,
                        out,
                    );
                }
                draw_footer(
                    self.battery,
                    self.battery_percent,
                    self.bolt,
                    self.shelf_clock,
                    out,
                );
                self.draw_info_card(out);
                if self.help {
                    shelf_ui::draw_help(&self.help_faces, &self.help_lines(), out);
                }
                if self.state.crt {
                    shelf_ui::draw_crt(out);
                }
            }
            Phase::About => {
                draw_backdrop(self.wallpaper, out);
                let card = self.member_card();
                let face = self.member_face.as_ref().filter(|(c, _)| Some(c) == card.as_ref());
                match (self.about_back, face) {
                    (false, Some((_, tex))) => shelf_ui::draw_member_card(Some(*tex), out),
                    _ => draw_sticker(self.sticker_face, out),
                }
                return;
            }
            Phase::Receipts { at } => {
                draw_backdrop(self.wallpaper, out);
                let under = self.receipts.len().saturating_sub(at + 1);
                shelf_ui::draw_receipt_spike(self.receipt_tex(), under, out);
                return;
            }
            Phase::Artwork | Phase::Search { .. } => {
                draw_backdrop(self.wallpaper, out);
                let faces = self
                    .artwork_faces
                    .as_ref()
                    .filter(|f| Some(&f.text) == self.screen_text().as_ref());
                shelf_ui::draw_artwork(faces, self.artwork_progress(), out);
                return;
            }
            // From the store the cart is seated over the dimmed shop.
            Phase::Inserting { cart, .. } if self.store() => {
                shelf_ui::draw_store_backdrop(
                    self.store_art,
                    shelf_ui::accent_of(self.selected_cart().map(|c| c.platform)),
                    out,
                );
                out.push(Draw::Rect {
                    x: 0.0,
                    y: -shelf_ui::band(),
                    w: OUT_W as f32,
                    h: OUT_H as f32 + shelf_ui::band(),
                    colour: [0.0, 0.0, 0.02, shelf_ui::BOX_DIM[3]],
                });
                self.chrome(cart, self.seat(), out);
            }
            // From the box shelf the cart is seated alone: the other boxes
            // went when it was taken out.
            Phase::Inserting { cart, .. } if self.boxes() => {
                draw_backdrop(self.wallpaper, out);
                self.chrome(cart, self.seat(), out);
            }
            Phase::Inserting { cart, .. } => {
                draw_backdrop(self.wallpaper, out);
                self.shelf()
                    .draw_row(Some(cart), 0.0, self.seat(), 1.0, out);
                self.chrome(cart, self.seat(), out);
            }
        }
        self.hud.draw(self.now(), out);
    }

    pub fn shelf_shake(&self) -> f32 {
        self.shake_at(self.now())
    }

    /// How far a refusal has pushed the shelf's carts sideways at `now`.
    pub fn shake_at(&self, now: Millis) -> f32 {
        if !self.on_shelf() {
            return 0.0;
        }
        self.refusal.map_or(0.0, |r| r.offset(now))
    }

    fn chrome(&self, stem: &str, dim: f32, out: &mut Vec<Draw>) {
        let Some((cart, face)) = self.shelf().find(stem) else {
            return;
        };
        let (rest, scale) = self.shelf().selected_at();
        SlotChrome {
            cart,
            face,
            rest,
            scale,
            seat: self.seat(),
            dim,
        }
        .draw(out);
    }

    pub fn set_shutdown_faces(&mut self, faces: Vec<(TexId, u32, u32)>) {
        self.shutdown_faces = faces;
    }

    pub fn cart_hints_made(&self) -> bool {
        self.cart_hints.is_some()
    }

    pub fn set_cart_hints_face(&mut self, tex: TexId, w: u32, h: u32) {
        self.cart_hints = Some((tex, w, h));
    }

    /// What the buttons do with the cart out of its box, fading in as it
    /// comes out.
    fn draw_cart_hints(&self, peek: shelf_ui::BoxView, out: &mut Vec<Draw>) {
        if let Some((tex, w, h)) = self.cart_hints {
            let alpha = (peek.openness(self.now()) - 0.6) / 0.4;
            shelf_ui::draw_cart_hints(tex, w, h, alpha, out);
        }
    }

    pub fn set_help_faces(&mut self, faces: Vec<(TexId, u32)>) {
        self.help_faces = faces;
    }

    pub fn display(&self) -> DisplayMode {
        self.state.display
    }

    pub fn sort(&self) -> SortKey {
        self.state.sort
    }

    fn store(&self) -> bool {
        self.state.display == DisplayMode::Store
    }

    fn boxes(&self) -> bool {
        self.state.display == DisplayMode::Boxes
    }

    /// The store, with loose carts on its shelf.
    fn store_carts(&self) -> bool {
        self.store() && self.state.store_carts
    }

    /// The store, with boxes on its shelf.
    fn store_boxes(&self) -> bool {
        self.store() && !self.state.store_carts
    }

    /// Whether the theme shows carts rather than boxes: the cart shelf, or
    /// the store's loose carts.
    pub fn view_carts(&self) -> bool {
        match self.state.display {
            DisplayMode::Carts => true,
            DisplayMode::Store => self.state.store_carts,
            DisplayMode::Boxes => false,
        }
    }

    /// The original theme: no rental shop's words on signs and stickers.
    pub fn plain(&self) -> bool {
        !self.store()
    }

    /// L2: swap between boxes and carts, in the theme chosen in Options.
    fn step_display(&mut self, now: Millis) {
        self.set_view(!self.view_carts(), now, true);
    }

    /// The Theme row of the menu: the video store, or the original
    /// shelves, keeping to carts or boxes as they were.
    fn set_theme(&mut self, store: bool, now: Millis) {
        let carts = self.view_carts();
        let mode = match (store, carts) {
            (true, _) => DisplayMode::Store,
            (false, true) => DisplayMode::Carts,
            (false, false) => DisplayMode::Boxes,
        };
        self.state.store_carts = carts;
        self.set_display(mode, now, false);
        // The signs and the shelf names change their words.
        self.resort_all();
    }

    /// L2, or the View row: carts or boxes, in the same theme.
    fn set_view(&mut self, carts: bool, now: Millis, announce: bool) {
        if self.view_carts() == carts {
            return;
        }
        match self.store() {
            true => self.toggle_store_shelf(now, announce),
            false => {
                let mode = match carts {
                    true => DisplayMode::Carts,
                    false => DisplayMode::Boxes,
                };
                self.set_display(mode, now, announce);
            }
        }
    }

    /// Swap the store's shelf between boxes and loose carts. The game in
    /// the middle stays in the middle.
    fn toggle_store_shelf(&mut self, now: Millis, announce: bool) {
        self.state.store_carts = !self.state.store_carts;
        self.box_view = None;
        self.drop_peek();
        self.shelf_mut().release_hold();
        self.shed_at = None;
        self.face_epoch += 1;
        if announce {
            self.hud
                .toast(Toast::StoreShelf(self.state.store_carts), now);
        }
        self.persist();
    }

    /// L2, or the Display row of the menu: show the shelves as carts or as
    /// boxes in the store. The game in the middle stays in the middle.
    fn set_display(&mut self, mode: DisplayMode, now: Millis, announce: bool) {
        if self.state.display == mode {
            return;
        }
        self.state.display = mode;
        self.box_view = None;
        self.drop_peek();
        self.blow = None;
        self.blow_rom = None;
        self.refusal = None;
        self.shelf_mut().release_hold();
        self.play_held = None;
        // The shelf's pictures are another kind now.
        self.shed_at = None;
        self.face_epoch += 1;
        // The sign swings down to announce the aisle, as it does on a new one.
        self.sign_at = now;
        if announce {
            self.hud.toast(Toast::Display(mode), now);
        }
        self.persist();
    }

    /// R2, or the Sort row of the menu: put every console's games in a new
    /// order, with the game in the middle still in the middle.
    fn set_sort(&mut self, key: SortKey, now: Millis, announce: bool) {
        self.state.sort = key;
        self.shelf_mut().release_hold();
        self.slot_letter = None;
        if key != SortKey::Name {
            self.ensure_details_loading();
        }
        self.resort_all();
        if announce {
            self.hud.toast(Toast::Sorted(key), now);
        }
        self.persist();
    }

    /// The saved sort, applied as the app starts: the details it needs for
    /// the shelf being shown are read first, so that shelf is never seen in
    /// the wrong order.
    fn apply_sort_state(&mut self) {
        if self.state.sort == SortKey::Name {
            return;
        }
        let at = self.shelf_at;
        if self.root.is_some() && !self.shelves[at].0.is_list() {
            let rows = self.read_details_of_shelf(at);
            self.accept_details(at, rows);
        }
        self.ensure_details_loading();
    }

    fn read_details_of_shelf(&self, at: usize) -> Vec<(PathBuf, Option<Details>)> {
        let Some(root) = &self.root else {
            return Vec::new();
        };
        self.shelves[at]
            .1
            .carts
            .iter()
            .map(|c| (c.rom.clone(), details_for(root, c)))
            .collect()
    }

    /// Start reading the details of every console's games, the shelf being
    /// shown first, if that is not already going on or done: the sorts
    /// other than by name go by them.
    fn ensure_details_loading(&mut self) {
        if self.details_rx.is_some() {
            return;
        }
        let Some(root) = self.root.clone() else {
            return;
        };
        let mut jobs: Vec<(usize, Vec<Cart>)> = self
            .shelves
            .iter()
            .enumerate()
            .filter(|(at, (key, shelf))| {
                !key.is_list() && !shelf.carts.is_empty() && !self.details_loaded[*at]
            })
            .map(|(at, (_, shelf))| (at, shelf.carts.clone()))
            .collect();
        if jobs.is_empty() {
            return;
        }
        let here = self.shelf_at;
        jobs.sort_by_key(|(at, _)| (*at != here, *at));
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("shelf-details".into())
            .spawn(move || {
                for (at, carts) in jobs {
                    let rows = carts
                        .iter()
                        .map(|c| (c.rom.clone(), details_for(&root, c)))
                        .collect();
                    if tx.send((at, rows)).is_err() {
                        return;
                    }
                }
            });
        if spawned.is_ok() {
            self.details_rx = Some(rx);
        }
    }

    /// Take in the details the thread has read, and put those shelves in
    /// order by them.
    fn poll_details(&mut self) {
        loop {
            let got = match &self.details_rx {
                Some(rx) => rx.try_recv(),
                None => return,
            };
            match got {
                Ok((at, rows)) => self.accept_details(at, rows),
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.details_rx = None;
                    return;
                }
            }
        }
    }

    fn accept_details(&mut self, at: usize, rows: Vec<(PathBuf, Option<Details>)>) {
        for (rom, details) in rows {
            self.details.entry(rom).or_insert(details);
        }
        if let Some(loaded) = self.details_loaded.get_mut(at) {
            *loaded = true;
        }
        self.resort_shelf(at);
    }

    /// The order a shelf is in now: by the sort, once its details are
    /// read, and by name until then.
    fn sort_in_use(&self, at: usize) -> SortKey {
        match self.details_loaded.get(at) {
            Some(true) => self.state.sort,
            _ => SortKey::Name,
        }
    }

    fn resort_all(&mut self) {
        for at in 0..self.shelves.len() {
            self.resort_shelf(at);
        }
    }

    /// Put a shelf in the order of the sort, keeping its selection on the
    /// same game, and work out its sections. The recently played shelf
    /// stays in the order the games were played in.
    fn resort_shelf(&mut self, at: usize) {
        if self.shelves[at].0.is_list() {
            return;
        }
        let key = self.sort_in_use(at);
        let plain = self.plain();
        let details = &self.details;
        let find = |c: &Cart| details.get(&c.rom).and_then(Option::as_ref);
        let (rentals, nextui) = (&self.rentals, &self.nextui_plays);
        let plays = |c: &Cart| {
            crate::game_logs::combined(rentals.get(&c.rom).copied(), nextui.get(&c.rom).copied())
        };
        // A shelf never put in another order is left as it was given.
        if key != SortKey::Name || self.shelf_sorted[at] {
            let order = sorted_order(&self.shelves[at].1.carts, find, plays, key);
            if order.iter().enumerate().any(|(n, i)| n != *i) {
                self.shelves[at].1.reorder(&order);
                self.shed_at = None;
                self.face_epoch += 1;
            }
        }
        self.shelf_sorted[at] = key != SortKey::Name;
        let labels = sections_worded(&self.shelves[at].1.carts, find, plays, key, plain);
        let groups = match key {
            SortKey::Name => Vec::new(),
            _ => {
                let mut id = 0u32;
                labels
                    .iter()
                    .enumerate()
                    .map(|(n, label)| {
                        if n == 0 || labels[n - 1] != *label {
                            id += 1;
                        }
                        id
                    })
                    .collect()
            }
        };
        self.shelves[at].1.set_groups(groups);
        self.section_labels[at] = labels;
    }

    /// The section the middle game is in under the sort, the aisle it
    /// stands in.
    pub fn current_section(&self) -> Option<String> {
        if self.shelf_key().is_list() {
            return Some(self.shelf_key().name_in(self.plain()).to_string());
        }
        self.section_labels
            .get(self.shelf_at)?
            .get(self.shelf().index)
            .cloned()
    }

    /// The section to print: for the sign in the store, and for a small tab
    /// over the game's name in the original theme.
    pub fn wanted_section(&self) -> Option<String> {
        if !matches!(self.phase, Phase::Shelf | Phase::Inserting { .. }) {
            return None;
        }
        self.current_section()
    }

    pub fn set_section_faces(&mut self, faces: SectionFaces) {
        self.section_faces = Some(faces);
    }

    pub fn section_faces(&self) -> Option<&SectionFaces> {
        self.section_faces.as_ref()
    }

    /// What a jump to the next stretch of the shelf landed on, to say in
    /// the slot: the letter when going by name, the section otherwise.
    fn jumped_to(&self) -> Option<String> {
        match self.state.sort {
            SortKey::Name => self
                .selected_stem()
                .map(|s| shelf_store::initial(s).to_string()),
            _ => self.current_section(),
        }
    }

    /// Note when the section the sign shows changes, so it swings.
    fn keep_sign(&mut self) {
        let now = self.now();
        let current = self.wanted_section();
        if current != self.sign_section {
            self.sign_section = current;
            self.sign_at = now;
        }
    }

    pub fn set_store_art(&mut self, backdrop: TexId) {
        self.store_art = Some(backdrop);
    }

    /// What the store's tag under cart `i` of the shelf says.
    fn stat_tag_of(&self, i: usize, middle: bool) -> Option<shelf_ui::StatTag> {
        let cart = self.shelf().carts.get(i)?;
        let offset = i64::from(self.state.utc_offset_min) * 60;
        let (times, last) = self
            .rental_of(&cart.rom)
            .map_or((0, 0), |r| (r.times, r.last + offset));
        let mut tag = shelf_ui::StatTag::new(times, last, middle);
        let time = shelf_ui::play_time_text(self.play_time_of(&cart.rom));
        if let (true, Some(time)) = (middle && times > 0, time) {
            tag.text = tag.text.replacen(" \u{b7} LAST", &format!(" \u{b7} {time} \u{b7} LAST"), 1);
        }
        Some(tag)
    }

    /// The tags the games around the middle hang, made or not.
    fn stat_tags_near(&self) -> Vec<shelf_ui::StatTag> {
        let shelf = self.shelf();
        let mut near = Vec::new();
        for slot in -1..=1 {
            let Some(i) = shelf.cart_at_offset(slot) else {
                continue;
            };
            let middles: &[bool] = if slot == 0 { &[true, false] } else { &[false] };
            near.extend(middles.iter().filter_map(|&m| self.stat_tag_of(i, m)));
        }
        near
    }

    /// Once many tags have been made, the ones no game near the middle
    /// hangs, for their textures to be used again.
    pub fn shed_stat_tags(&mut self) -> Vec<TexId> {
        if self.stat_tags.len() <= STAT_TAGS_KEPT {
            return Vec::new();
        }
        let near = self.stat_tags_near();
        let mut shed = Vec::new();
        self.stat_tags.retain(|tag, (tex, _, _)| {
            let keep = near.contains(tag);
            if !keep {
                shed.push(*tex);
            }
            keep
        });
        shed
    }

    /// The store's tags still to be made for the games on screen.
    pub fn wanted_stat_tags(&self) -> Vec<shelf_ui::StatTag> {
        if !self.store() || !self.on_shelf() {
            return Vec::new();
        }
        let shelf = self.shelf();
        let mut want = Vec::new();
        for slot in -1..=1 {
            let Some(i) = shelf.cart_at_offset(slot) else {
                continue;
            };
            // The middle game's long tag, and its short one for when it is
            // scrolled aside.
            let middles: &[bool] = if slot == 0 { &[true, false] } else { &[false] };
            for &middle in middles {
                if let Some(tag) = self.stat_tag_of(i, middle) {
                    if !self.stat_tags.contains_key(&tag) && !want.contains(&tag) {
                        want.push(tag);
                    }
                }
            }
        }
        want
    }

    pub fn set_stat_tag_face(&mut self, tag: shelf_ui::StatTag, face: (TexId, u32, u32)) {
        self.stat_tags.insert(tag, face);
    }

    /// Whether the store's pictures are wanted and not made yet.
    pub fn wants_store_art(&self) -> bool {
        self.store() && self.store_art.is_none()
    }

    /// The store: its shelf of boxes or loose carts under the aisle sign and
    /// the game's name. Opening the middle box (or picking up the middle
    /// cart) dims the store and brings the console up from below, and the
    /// cart comes down to rest over its slot.
    fn draw_store_scene(&self, out: &mut Vec<Draw>) {
        let now = self.now();
        let ceiling = shelf_ui::store_ceiling();
        let carts = self.state.store_carts;
        let shake = self.shelf_shake();
        let shelf = self.shelf();
        let platform = self.selected_cart().map(|c| c.platform);
        shelf_ui::draw_store_backdrop(self.store_art, shelf_ui::accent_of(platform), out);
        let tag = |i: usize, middle: bool| {
            let tag = self.stat_tag_of(i, middle)?;
            self.stat_tags.get(&tag).copied()
        };
        let open = self.peek.map_or(0.0, |p| p.openness(now));
        let home = shelf.store_selected_rect(ceiling, carts);
        // A loose cart turned round to its box leaves its place empty.
        let turned = self.box_view.filter(|v| v.cart_squash(now) == 0.0);
        if self.peek.is_none() && turned.is_none() {
            self.draw_note_behind(home, out);
        }
        let blow = self.blow().map(|b| (b, now));
        let hide = self.peek.is_some() || turned.is_some();
        shelf.draw_store_row(ceiling, carts, &tag, hide, 1.0 - open, shake, blow, out);
        shelf_ui::draw_store_band(out);
        if let Some(faces) = self
            .section_faces
            .as_ref()
            .filter(|f| Some(&f.label) == self.sign_section.as_ref())
        {
            shelf_ui::draw_sign(Some(faces.sign), now.saturating_sub(self.sign_at), out);
        }
        let Some(peek) = self.peek else {
            self.draw_game_name(out);
            if let (Some(view), Some(home)) = (self.box_view, home) {
                let (face, back, details, plain) = self.box_view_faces();
                view.draw(
                    now,
                    home,
                    shelf.selected_face(),
                    face,
                    back,
                    details,
                    plain,
                    out,
                );
            }
            return;
        };
        // The store dims and the console comes up from under the screen.
        out.push(Draw::Rect {
            x: 0.0,
            y: -shelf_ui::band(),
            w: OUT_W as f32,
            h: OUT_H as f32 + shelf_ui::band(),
            colour: [0.0, 0.0, 0.02, shelf_ui::BOX_DIM[3] * open],
        });
        shelf_ui::draw_risen_slot(platform, open, out);
        self.draw_game_name(out);
        let rest = shelf.selected_rect();
        match (home, rest) {
            _ if peek.taken_out(now) => {
                match self.blow() {
                    Some(blow) => shelf.draw_middle_blown(shake, &blow, now, out),
                    None => shelf.draw_middle(shake, out),
                }
                self.draw_note_on_cart(out);
            }
            (Some(home), Some(rest)) if carts => {
                shelf.draw_store_lift(home, rest, open, out);
            }
            (Some(home), Some(rest)) => {
                let front = shelf.cart_at_offset(0).and_then(|i| shelf.tall_face(i));
                let (_, back, details, plain) = self.box_view_faces();
                peek.draw_peek(
                    now,
                    home,
                    rest,
                    shelf.selected_face(),
                    front.map(|f| f.0),
                    back,
                    details,
                    plain,
                    ceiling,
                    out,
                );
            }
            _ => {}
        }
        self.draw_cart_hints(peek, out);
        draw_slot_name(self.shelf_platform, self.slot_name_alpha(), out);
    }

    /// The box shelf: the boxes in the dark under the game's name, or the
    /// middle one opened and its cart coming out of it, and the name and
    /// details over everything.
    fn draw_boxes_scene(&self, out: &mut Vec<Draw>) {
        let now = self.now();
        let ceiling = boxes_ceiling();
        let shake = self.shelf_shake();
        let shelf = self.shelf();
        let platform = self.selected_cart().map(|c| c.platform);
        draw_backdrop(self.wallpaper, out);
        match self.peek {
            Some(peek) if peek.taken_out(now) => {
                match self.blow() {
                    Some(blow) => shelf.draw_middle_blown(shake, &blow, now, out),
                    None => shelf.draw_middle(shake, out),
                }
                self.draw_note_on_cart(out);
            }
            Some(peek) => {
                shelf.draw_boxes_row(ceiling, true, 1.0 - peek.openness(now), shake, out);
                shelf_ui::draw_empty_slot_for(platform, out);
                let front = shelf.cart_at_offset(0).and_then(|i| shelf.tall_face(i));
                let (_, back, details, plain) = self.box_view_faces();
                if let (Some(home), Some(rest)) =
                    (shelf.boxes_selected_rect(ceiling), shelf.selected_rect())
                {
                    peek.draw_peek(
                        now,
                        home,
                        rest,
                        shelf.selected_face(),
                        front.map(|f| f.0),
                        back,
                        details,
                        plain,
                        ceiling,
                        out,
                    );
                }
            }
            None => {
                self.draw_note_behind(shelf.boxes_selected_rect(ceiling), out);
                shelf.draw_boxes_row(ceiling, false, 1.0, shake, out);
                shelf_ui::draw_empty_slot_for(platform, out);
            }
        }
        if let Some(peek) = self.peek {
            self.draw_cart_hints(peek, out);
        }
        self.draw_game_name(out);
        self.draw_section_tab(out);
        draw_slot_name(self.shelf_platform, self.slot_name_alpha(), out);
    }

    /// The cart shelf's small tab naming the section, over the game's name.
    fn draw_section_tab(&self, out: &mut Vec<Draw>) {
        let Some(label) = self.wanted_section() else {
            return;
        };
        let Some(faces) = self.section_faces.as_ref().filter(|f| f.label == label) else {
            return;
        };
        let Some(cart) = self.selected_cart() else {
            return;
        };
        let (name_y, _) = self.name_place(cart);
        // Snug over the name, in the room the name's own face leaves above it.
        let y = name_y - shelf_ui::TAB_FACE_H as f32 + 2.0;
        if y < -shelf_ui::band() - 0.5 {
            return;
        }
        let age = self.now().saturating_sub(self.sign_at) as f32 / shelf_ui::SIGN_IN_MS as f32;
        shelf_ui::draw_tab(Some(faces.tab), y, 0.55 * age.min(1.0), out);
    }

    /// The pictures box view is drawn with: the front, the back, the
    /// details strip or back, and the colour of a plain side.
    #[allow(clippy::type_complexity)]
    fn box_view_faces(
        &self,
    ) -> (
        Option<(TexId, u32, u32)>,
        Option<(TexId, u32, u32)>,
        Option<(TexId, u32, u32)>,
        [f32; 4],
    ) {
        let front = self
            .box_face
            .as_ref()
            .filter(|f| Some(&f.0) == self.box_art.as_ref())
            .map(|f| (f.1, f.2, f.3));
        let back = self
            .box_back_face
            .as_ref()
            .filter(|f| Some(&f.0) == self.box_back.as_ref())
            .map(|f| (f.1, f.2, f.3));
        let plain = shelf_ui::slot_colours(self.selected_cart().map(|c| c.platform))[0];
        let made = self.back_details_key();
        let details = self
            .back_details_face
            .as_ref()
            .filter(|f| Some(&f.0) == made.as_ref())
            .map(|f| (f.1, f.2, f.3));
        (front, back, details, plain)
    }

    fn on_shelf(&self) -> bool {
        matches!(self.phase, Phase::Shelf)
    }

    pub fn browsed_rom(&self) -> Option<&std::path::Path> {
        if !self.on_shelf() {
            return None;
        }
        let shelf = self.shelf();
        shelf.carts.get(shelf.index).map(|c| c.rom.as_path())
    }

    /// Fill the recently played shelf with these roms, newest first, from
    /// the carts on the console shelves.
    pub fn set_recent(&mut self, roms: &[PathBuf]) {
        let carts: Vec<Cart> = roms
            .iter()
            .filter_map(|rom| self.carts().find(|c| c.rom == *rom).cloned())
            .take(RECENT_MAX)
            .collect();
        if let Some((_, shelf)) = self
            .shelves
            .iter_mut()
            .find(|(k, _)| *k == ShelfKey::Recent)
        {
            *shelf = Shelf::new(carts);
        }
        self.refresh_dust();
    }

    /// Choose the Staff Picks: games left with a save and not played for
    /// a few days, then unplayed games in the genre played most, then the
    /// best rated games never played; ten at most, turning over each day.
    /// Left alone while it is the shelf being shown.
    pub fn refresh_picks(&mut self) {
        let Some(at) = self.shelves.iter().position(|(k, _)| *k == ShelfKey::Picks) else {
            return;
        };
        if self.shelf_at == at && !self.shelves[at].1.carts.is_empty() {
            return;
        }
        let now = self.utc_secs();
        let day = (now / 86_400).max(0) as usize;
        let saves = self.save_slots();
        let carts: Vec<&Cart> = self.carts().collect();
        let plays = |c: &Cart| self.rental_of(&c.rom);
        let facts = |c: &Cart| self.pick_facts.get(&c.rom);
        let genres = |c: &Cart| facts(c).map_or(&[][..], |f| &f.1[..]);
        let unplayed = |c: &Cart| plays(c).is_none_or(|r| r.times == 0);
        let rating = |c: &Cart| facts(c).and_then(|f| f.0).unwrap_or(0.0);
        let mut picked: Vec<PathBuf> = Vec::new();
        let take = |list: Vec<&Cart>, most: usize, picked: &mut Vec<PathBuf>, turn: bool| {
            let fresh: Vec<&&Cart> = list.iter().filter(|c| !picked.contains(&c.rom)).collect();
            let start = if turn && !fresh.is_empty() { day % fresh.len() } else { 0 };
            for c in fresh.iter().cycle().skip(start).take(most.min(fresh.len())) {
                picked.push(c.rom.clone());
            }
        };
        // Left part way: a save, and no play for three days.
        let mut left: Vec<&Cart> = carts
            .iter()
            .copied()
            .filter(|c| {
                c.rom.file_name().is_some_and(|n| saves.contains(n))
                    && plays(c).is_some_and(|r| now - r.last >= 3 * 86_400)
            })
            .collect();
        left.sort_by_key(|c| plays(c).map_or(0, |r| r.last));
        take(left, 4, &mut picked, false);
        // The genre played most, by plays.
        let mut most: HashMap<&str, u32> = HashMap::new();
        for c in &carts {
            let (Some(g), Some(r)) = (genres(c).first(), plays(c)) else {
                continue;
            };
            *most.entry(g.as_str()).or_insert(0) += r.times;
        }
        let top = most.into_iter().max_by_key(|(g, n)| (*n, std::cmp::Reverse(*g)));
        if let Some((genre, _)) = top {
            let mut same: Vec<&Cart> = carts
                .iter()
                .copied()
                .filter(|c| {
                    unplayed(c) && genres(c).iter().any(|g| g == genre)
                })
                .collect();
            same.sort_by(|a, b| rating(b).total_cmp(&rating(a)));
            take(same, 3, &mut picked, true);
        }
        // The best rated never played.
        let mut rated: Vec<&Cart> = carts
            .iter()
            .copied()
            .filter(|c| unplayed(c) && rating(c) >= 3.5)
            .collect();
        rated.sort_by(|a, b| rating(b).total_cmp(&rating(a)));
        let room = PICKS_MAX.saturating_sub(picked.len());
        take(rated, room, &mut picked, true);
        let fresh: Vec<Cart> = picked
            .iter()
            .filter_map(|rom| carts.iter().find(|c| c.rom == *rom).map(|c| (*c).clone()))
            .collect();
        if fresh.iter().map(|c| &c.rom).eq(self.shelves[at].1.carts.iter().map(|c| &c.rom)) {
            return;
        }
        self.shelves[at].1 = Shelf::new(fresh);
        self.face_epoch += 1;
        self.refresh_dust();
    }

    /// Read every game's rating and genres off to the side, for the picks.
    fn read_pick_facts(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let carts: Vec<Cart> = self.all_carts.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("shelf-picks".into())
            .spawn(move || {
                let facts = carts
                    .iter()
                    .filter_map(|c| {
                        let d = details_for(&root, c)?;
                        Some((c.rom.clone(), d.rating, d.genres))
                    })
                    .collect();
                let _ = tx.send(facts);
            });
        if spawned.is_ok() {
            self.picks_rx = Some(rx);
        }
    }

    fn poll_picks(&mut self) {
        let Some(rx) = &self.picks_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(facts) => {
                self.picks_rx = None;
                for (rom, rating, genres) in facts {
                    self.pick_facts.insert(rom, (rating, genres));
                }
                self.refresh_picks();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.picks_rx = None,
        }
    }

    /// The rom file names NextUI has a save state for, read from its
    /// `.minui/<tag>/` folders.
    fn save_slots(&self) -> std::collections::HashSet<std::ffi::OsString> {
        let mut out = std::collections::HashSet::new();
        let Some((shared, _)) = &self.resume_lookup else {
            return out;
        };
        let Ok(tags) = std::fs::read_dir(shared.join(".minui")) else {
            return out;
        };
        for tag in tags.flatten() {
            let Ok(files) = std::fs::read_dir(tag.path()) else {
                continue;
            };
            for f in files.flatten() {
                let name = f.file_name();
                let Some(rom) = name.to_str().and_then(|n| n.strip_suffix(".txt")) else {
                    continue;
                };
                if f.metadata().is_ok_and(|m| m.len() > 0) {
                    out.insert(std::ffi::OsString::from(rom));
                }
            }
        }
        out
    }

    /// Fill the favourites shelf with these roms, newest first, from the carts
    /// on the console shelves.
    pub fn set_favourites(&mut self, roms: &[PathBuf]) {
        let carts: Vec<Cart> = roms
            .iter()
            .filter_map(|rom| self.carts().find(|c| c.rom == *rom).cloned())
            .collect();
        let Some(at) = self
            .shelves
            .iter()
            .position(|(k, _)| *k == ShelfKey::Favourites)
        else {
            return;
        };
        let index = self.shelves[at].1.index.min(carts.len().saturating_sub(1));
        self.shelves[at].1 = Shelf::new(carts);
        self.shelves[at].1.index = index;
        self.shelves[at].1.scroll = index as f32;
        self.face_epoch += 1;
        if self.shelf_at == at && self.shelves[at].1.carts.is_empty() {
            if let Some(to) = self.next_shelf(1) {
                self.switch_shelf_to(to);
            }
        }
        self.refresh_dust();
    }

    /// The favourite roms, newest first.
    pub fn favourites(&self) -> Vec<PathBuf> {
        self.shelves
            .iter()
            .find(|(k, _)| *k == ShelfKey::Favourites)
            .map(|(_, s)| s.carts.iter().map(|c| c.rom.clone()).collect())
            .unwrap_or_default()
    }

    /// Add the game in the middle to the favourites, or take it off them.
    fn toggle_favourite(&mut self, now: Millis) {
        let Some(rom) = self
            .shelf()
            .carts
            .get(self.shelf().index)
            .map(|c| c.rom.clone())
        else {
            return;
        };
        self.play_held = None;
        // From the file, so a favourite whose game is missing from this scan
        // (a card not mounted, a rom renamed) is kept for when it is back.
        let mut roms = match &self.root {
            Some(root) => read_favourites(root),
            None => self.favourites(),
        };
        let added = !roms.contains(&rom);
        match added {
            true => roms.insert(0, rom),
            false => {
                roms.retain(|r| *r != rom);
                if self.shelf_key() == ShelfKey::Favourites {
                    self.drop_peek();
                }
            }
        }
        self.set_favourites(&roms);
        self.hud.toast(Toast::Favourite(added), now);
        let Some(root) = &self.root else { return };
        let text: String = roms.iter().map(|r| format!("{}\n", r.display())).collect();
        let path = favourites_path(root);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = shelf_store::atomic_write(&path, text.as_bytes()) {
            eprintln!("shelf: favourites: {}: {e}", path.display());
        }
    }

    /// Whether the game in the middle is one hidden from the shelves, shown
    /// because hidden games are.
    pub fn selected_hidden(&self) -> bool {
        self.shelf()
            .carts
            .get(self.shelf().index)
            .is_some_and(|c| self.hidden.contains(&c.rom))
    }

    /// Menu, Hide Game: take the game in the middle off the shelves, or put
    /// it back when it is a hidden one being shown.
    fn toggle_hidden(&mut self) {
        self.close_quick_menu();
        let Some(rom) = self
            .shelf()
            .carts
            .get(self.shelf().index)
            .map(|c| c.rom.clone())
        else {
            return;
        };
        self.play_held = None;
        self.drop_peek();
        match self.hidden.contains(&rom) {
            true => self.hidden.retain(|r| *r != rom),
            false => self.hidden.push(rom),
        }
        self.apply_hidden();
        let Some(root) = &self.root else { return };
        let text: String = self
            .hidden
            .iter()
            .map(|r| format!("{}\n", r.display()))
            .collect();
        let path = hidden_path(root);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = shelf_store::atomic_write(&path, text.as_bytes()) {
            eprintln!("shelf: hidden: {}: {e}", path.display());
        }
    }

    /// Build the console shelves again without the hidden games, or with
    /// them while they are shown, each shelf staying on the game it was on
    /// (or the one after it, when that game is the one hidden).
    fn apply_hidden(&mut self) {
        let show = self.state.show_hidden;
        let mut changed = false;
        for at in 0..self.shelves.len() {
            let ShelfKey::Platform(p) = self.shelves[at].0 else {
                continue;
            };
            let hidden = &self.hidden;
            let wanted: Vec<Cart> = self
                .all_carts
                .iter()
                .filter(|c| c.platform == p && (show || !hidden.contains(&c.rom)))
                .cloned()
                .collect();
            let shelf = &self.shelves[at].1;
            if wanted.len() == shelf.carts.len()
                && wanted
                    .iter()
                    .all(|c| shelf.carts.iter().any(|d| d.rom == c.rom))
            {
                continue;
            }
            let was = shelf.carts.get(shelf.index).map(|c| c.rom.clone());
            let index = shelf.index;
            let mut fresh = Shelf::new(wanted);
            let i = was
                .and_then(|rom| fresh.carts.iter().position(|c| c.rom == rom))
                .unwrap_or(index.min(fresh.carts.len().saturating_sub(1)));
            fresh.select(i);
            self.shelves[at].1 = fresh;
            self.shelf_sorted[at] = false;
            self.resort_shelf(at);
            changed = true;
        }
        if !changed {
            return;
        }
        self.face_epoch += 1;
        self.shed_at = None;
        let lists = self
            .root
            .as_deref()
            .map(|root| (read_recent(root), read_favourites(root)));
        if let Some((recent, favourites)) = lists {
            self.set_recent(&recent);
            self.set_favourites(&favourites);
        }
        self.refresh_picks();
        if self.shelf().carts.is_empty() {
            if let Some(to) = self.next_shelf(1) {
                self.switch_shelf_to(to);
            }
        }
        self.name_pending = true;
        self.refresh_dust();
    }

    pub fn shelf_key(&self) -> ShelfKey {
        self.shelves[self.shelf_at].0
    }

    /// Put the rom at the front of the recently played list on the card.
    fn remember_played(&self, rom: &Path) {
        let Some(root) = &self.root else { return };
        let mut roms = read_recent(root);
        roms.retain(|r| r != rom);
        roms.insert(0, rom.to_path_buf());
        roms.truncate(RECENT_MAX);
        let text: String = roms.iter().map(|r| format!("{}\n", r.display())).collect();
        let path = recent_path(root);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("shelf: recent: {}: {e}", path.display());
        }
    }

    fn insert(&mut self) {
        self.fresh = false;
        if !self.on_shelf() {
            return;
        }
        let at = self.shelf().index;
        if let Some(rom) = self.shelf().carts.get(at).map(|c| c.rom.clone()) {
            self.remember_played(&rom);
            self.mark_played(&rom);
            self.mark_rented(&rom);
        }
        let Some(cart) = self.shelf_mut().carts.get_mut(at).map(|c| {
            c.read_header();
            c.stem.clone()
        }) else {
            return;
        };
        eprintln!(
            "shelf: launch: insert {cart} at {:.1} ms",
            crate::boot_time::ms()
        );
        self.play_held = None;
        self.refusal = None;
        self.phase = Phase::Inserting { cart, t: 0.0 };
    }

    fn begin_power_off(&mut self) {
        if self.powering_off {
            return;
        }
        self.powering_off = true;
        self.act_at = self.now() + SHUTDOWN_SHOW_MS;
    }

    pub fn on_battery(&mut self, b: Battery) {
        if b.percent > BATTERY_CRITICAL || self.powering_off {
            return;
        }
        if matches!(b.charge, Charge::Charging | Charge::Full) {
            return;
        }
        self.begin_power_off();
    }

    pub fn settle_saves(&mut self) {
        self.state_writer.flush();
    }

    pub fn set_icon_faces(&mut self, faces: Vec<TexId>) {
        self.hud.set_icons(faces);
    }

    pub fn set_toast_faces(&mut self, faces: Vec<TexId>) {
        self.hud.set_toasts(faces);
    }

    pub fn toast(&self) -> Option<Toast> {
        self.hud.said(self.now())
    }

    pub fn card_alarm(&self) -> bool {
        self.hud.alarmed()
    }
}

/// An app being built on another thread.
pub enum Booting {
    Thread(std::thread::JoinHandle<App>, PathBuf),
    Inline(PathBuf),
}

impl Booting {
    /// The app, waiting for it if it is not ready; built here instead if its
    /// thread could not start or died.
    pub fn finish(self) -> App {
        match self {
            Booting::Thread(handle, root) => handle.join().unwrap_or_else(|_| App::boot(&root)),
            Booting::Inline(root) => App::boot(&root),
        }
    }
}

/// Read the header of every Game Boy rom whose cart shape depends on it,
/// four at a time, so the first look at the shelves does not read them
/// one by one.
fn warm_headers(carts: &[Cart]) {
    let todo: Vec<&Cart> = carts
        .iter()
        .filter(|c| c.platform == Platform::Gb && c.shell.is_none())
        .collect();
    if todo.len() < 2 {
        for cart in todo {
            let _ = shelf_store::gb::header_cached(&cart.rom);
        }
        return;
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(cart) = todo.get(i) else {
                    return;
                };
                let _ = shelf_store::gb::header_cached(&cart.rom);
            });
        }
    });
}

fn up(level: u8, step: u8, max: u8) -> u8 {
    level.saturating_add(step).min(max)
}

fn clock_screen(utc: i64, offset_min: i16, from_menu: bool) -> Phase {
    Phase::SetClock {
        picker: ClockPicker::local(utc, offset_min),
        seed: utc - utc.rem_euclid(60),
        from_menu,
    }
}

fn system_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The tag of the system a rom is in, such as `GBA` from
/// `Roms/Game Boy Advance (GBA)/Game.gba`: the "(TAG)" ending the folder
/// directly under the Roms folder.
fn system_tag(rom: &Path, roms: &Path) -> Option<String> {
    let folder = rom
        .strip_prefix(roms)
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()?;
    let inner = folder.strip_suffix(')')?;
    let (_, tag) = inner.rsplit_once('(')?;
    (!tag.is_empty() && !tag.contains(['(', ')'])).then(|| tag.to_string())
}

/// NextUI's box art for a cart: the picture in `.media` beside its rom.
pub fn box_art_of(cart: &Cart) -> Option<PathBuf> {
    if let Some(label) = cart
        .label
        .as_deref()
        .filter(|p| p.components().any(|c| c.as_os_str() == ".media"))
    {
        return Some(label.to_path_buf());
    }
    let path = cart
        .rom
        .parent()?
        .join(".media")
        .join(format!("{}.png", cart.stem));
    path.is_file().then_some(path)
}

/// A number that differs from one press to the next, for picking a game.
fn random_u64() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    h.finish()
}

/// What a made box back is for: the game, which kind of back, how big,
/// and the colour of the panel under it.
#[derive(Clone, PartialEq, Debug)]
pub struct BackKey {
    pub stem: String,
    /// A strip along the foot of a back scan, rather than a whole back.
    pub strip: bool,
    pub w: u32,
    pub h: u32,
    pub panel: [f32; 4],
    /// The rental sticker, so that a play since it was made remakes it.
    pub rental: Option<shelf_ui::RentalSticker>,
}

/// What an info card is made for: the game, its rental sticker, so that
/// a play since it was made remakes it, and its screenshots.
#[derive(Clone, PartialEq, Debug)]
pub struct InfoKey {
    pub rom: PathBuf,
    pub rental: Option<shelf_ui::RentalSticker>,
    /// How many screenshots it has, so that Get Artwork adding them
    /// remakes it.
    pub shots: usize,
}

/// The info card coming up in front of a box's back, from `since`, or
/// going away again.
#[derive(Copy, Clone, Debug, PartialEq)]
struct InfoCard {
    since: Millis,
    away: bool,
}

impl InfoCard {
    /// How far up it is: 0 gone, 1 in place.
    fn shown(&self, now: Millis) -> f32 {
        let t = (now.saturating_sub(self.since) as f32 / shelf_ui::INFO_MS as f32).min(1.0);
        if self.away {
            1.0 - t
        } else {
            t
        }
    }

    /// Going away from wherever it has got to.
    fn closed(self, now: Millis) -> Self {
        let back = ((1.0 - self.shown(now)) * shelf_ui::INFO_MS as f32) as Millis;
        InfoCard {
            since: now.saturating_sub(back),
            away: true,
        }
    }
}

/// A screenshot Get Artwork saved for the info card,
/// `Shots/<console>/<rom name>.<kind>.jpg`, or a PNG put there instead.
fn shot_of(root: &Path, cart: &Cart, kind: &str) -> Option<PathBuf> {
    let dir = root.join("Shots").join(cart.platform.dir_name());
    ["jpg", "png"]
        .into_iter()
        .map(|ext| dir.join(format!("{}.{kind}.{ext}", cart.stem)))
        .find(|p| p.is_file())
}

/// A scan of the back of a cart's box, put in the card's Boxes folder as
/// `Boxes/<console>/<rom name>.back.png`.
/// A full-size scan of the front of a game's box, kept beside the backs,
/// for the box shelf to draw large.
pub fn box_front_of(root: &Path, cart: &Cart) -> Option<PathBuf> {
    box_scan_of(root, cart, "front")
}

fn box_back_of(root: &Path, cart: &Cart) -> Option<PathBuf> {
    box_scan_of(root, cart, "back")
}

/// `Boxes/<console>/<rom name>.<side>.png`, or the `.jpg` Get Artwork
/// saves, which is smaller on the card.
fn box_scan_of(root: &Path, cart: &Cart, side: &str) -> Option<PathBuf> {
    let dir = root.join("Boxes").join(cart.platform.dir_name());
    ["png", "jpg"]
        .into_iter()
        .map(|ext| dir.join(format!("{}.{side}.{ext}", cart.stem)))
        .find(|p| p.is_file())
}

/// Seconds on the clock that stops while the Brick sleeps, so a game left
/// paused in a sleeping Brick does not count as played.
fn monotonic_secs() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime only writes the timespec it is given.
    let ok = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) } == 0;
    if ok {
        ts.tv_sec.max(0) as u64
    } else {
        0
    }
}
