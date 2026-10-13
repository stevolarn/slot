//! Get Artwork: box fronts, box backs, cart photos and game details for
//! the games on the shelves, from the LaunchBox Games Database, fetched on
//! the Brick itself.
//!
//! LaunchBox's list of games is a 108 MB zip. It is never saved: the one
//! file in it that is needed is read in ranges from the server and
//! unpacked as it arrives, and only what the games on the card need is
//! kept, in a small index (`Config/launchbox.txt`) so that later runs can
//! go straight to the pictures. Box scans are saved as JPEG at the size
//! Shelf draws them, about 90 KB each.

pub mod catalog;
pub mod names;
pub mod net;
pub mod picture;
pub mod zip;

use std::collections::HashMap;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub use catalog::{Found, Game};
pub use net::{Net, NetError, Web};
use shelf_store::Platform;

/// The index of what LaunchBox has for the card's games, under `Config`.
pub const INDEX_FILE: &str = "launchbox.txt";
/// After this long the index is read again from LaunchBox, which may have
/// gained pictures since.
const INDEX_MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);
/// Left free on the card: fetching stops before the card gets fuller.
pub const SPACE_MARGIN: u64 = 200 << 20;
/// About how big each kind of picture is, saved, for the estimate.
const BOX_BYTES: u64 = 90 << 10;
const CART_BYTES: u64 = 130 << 10;
/// Between pictures, to go easy on the image server.
pub const PAUSE: Duration = Duration::from_millis(300);
const METADATA_ENTRY: &str = "Metadata.xml";

/// The kinds of picture, where each is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Front,
    Back,
    Cart,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Front, Kind::Back, Kind::Cart];

    /// Where Get Artwork saves this picture of a game.
    pub fn path(self, root: &Path, (platform, stem): &Game) -> PathBuf {
        let tag = platform.dir_name();
        match self {
            Kind::Front => root.join("Boxes").join(tag).join(format!("{stem}.front.jpg")),
            Kind::Back => root.join("Boxes").join(tag).join(format!("{stem}.back.jpg")),
            Kind::Cart => root.join("Carts").join(tag).join(format!("{stem}.png")),
        }
    }

    /// Whether the card has this picture of a game already, in the form
    /// Get Artwork saves or the PNG the computer tool and installer send.
    pub fn on_card(self, root: &Path, game: &Game) -> bool {
        let path = self.path(root, game);
        path.is_file() || path.with_extension("png").is_file()
    }

    fn bytes(self) -> u64 {
        match self {
            Kind::Cart => CART_BYTES,
            _ => BOX_BYTES,
        }
    }
}

/// Where a game's details file goes.
pub fn details_path(root: &Path, (platform, stem): &Game) -> PathBuf {
    root.join(shelf_store::DETAILS_DIR)
        .join(platform.dir_name())
        .join(format!("{stem}.txt"))
}

/// Bytes free on the card that holds `root`.
pub fn free_space(root: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(root.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: a valid C string and a statvfs to fill.
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    Some(st.f_bavail as u64 * st.f_frsize as u64)
}

/// How many pictures the games still lack, and about how much room they
/// would take: at most, as LaunchBox does not have every picture.
pub fn estimate(root: &Path, games: &[Game]) -> (usize, u64) {
    let mut n = 0;
    let mut bytes = 0;
    for game in games {
        for kind in Kind::ALL {
            if !kind.on_card(root, game) {
                n += 1;
                bytes += kind.bytes();
            }
        }
    }
    (n, bytes)
}

/// For each console with games, how many lack a box front, a box back and
/// a cart photo, in that order.
pub fn missing_by_console(root: &Path, games: &[Game]) -> Vec<(Platform, [usize; 3])> {
    let mut out: Vec<(Platform, [usize; 3])> = Vec::new();
    for game in games {
        let at = match out.iter().position(|(p, _)| *p == game.0) {
            Some(at) => at,
            None => {
                out.push((game.0, [0; 3]));
                out.len() - 1
            }
        };
        for (i, kind) in Kind::ALL.into_iter().enumerate() {
            if !kind.on_card(root, game) {
                out[at].1[i] += 1;
            }
        }
    }
    out.sort_by_key(|(p, _)| Platform::ALL.iter().position(|q| q == p));
    out
}

/// One line of the index: what LaunchBox has for a game, or `None` when
/// it does not have the game.
type Entry = Option<[Option<String>; 3]>;

fn index_path(root: &Path) -> PathBuf {
    root.join("Config").join(INDEX_FILE)
}

/// The index: for each game, the file names of its pictures on LaunchBox.
/// Lines are `TAG<tab>game<tab>front<tab>back<tab>cart`, `-` for none, or
/// `TAG<tab>game<tab>?` for a game LaunchBox does not have.
pub fn read_index(root: &Path) -> HashMap<Game, Entry> {
    let Ok(text) = std::fs::read_to_string(index_path(root)) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let parts: Vec<&str> = line.split('\t').collect();
        let Some(platform) = parts
            .first()
            .and_then(|t| Platform::ALL.into_iter().find(|p| p.dir_name() == *t))
        else {
            continue;
        };
        let Some(stem) = parts.get(1) else {
            continue;
        };
        let file = |i: usize| {
            parts
                .get(i)
                .filter(|f| !f.is_empty() && **f != "-")
                .map(|f| f.to_string())
        };
        let entry = match parts.get(2) {
            Some(&"?") => None,
            _ => Some([file(2), file(3), file(4)]),
        };
        out.insert((platform, stem.to_string()), entry);
    }
    out
}

fn write_index(root: &Path, index: &HashMap<Game, Entry>) -> io::Result<()> {
    let mut lines: Vec<String> = index
        .iter()
        // A name the format cannot hold is left out, and looked up again.
        .filter(|((_, stem), _)| !stem.contains(['\t', '\n', '\r']))
        .map(|((p, stem), e)| match e {
            None => format!("{}\t{stem}\t?", p.dir_name()),
            Some(files) => {
                let f = |i: usize| files[i].clone().unwrap_or_else(|| "-".into());
                format!("{}\t{stem}\t{}\t{}\t{}", p.dir_name(), f(0), f(1), f(2))
            }
        })
        .collect();
    lines.sort();
    let text = format!(
        "# What the LaunchBox Games Database has for the games on this card, made by Get Artwork.\n{}\n",
        lines.join("\n")
    );
    let path = index_path(root);
    std::fs::create_dir_all(path.parent().unwrap_or(root))?;
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, path)
}

/// Whether the index is recent. One dated in the future, as it is when
/// the Brick's clock was set back after it was written, counts as recent,
/// rather than making every run read LaunchBox's whole list again.
fn index_fresh(root: &Path) -> bool {
    let Ok(written) = std::fs::metadata(index_path(root)).and_then(|m| m.modified()) else {
        return false;
    };
    SystemTime::now()
        .duration_since(written)
        .map_or(true, |age| age < INDEX_MAX_AGE)
}

/// Where a run is.
#[derive(Clone, Debug, PartialEq)]
pub enum Stage {
    /// Reading LaunchBox's list of games: compressed bytes read of all.
    Reading { read: u64, of: u64 },
    /// Fetching pictures: done of all.
    Fetching { done: usize, of: usize },
    Finished(Summary),
    Failed(String),
}

/// How a run ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub pictures: usize,
    pub details: usize,
    /// Games LaunchBox does not have.
    pub unknown: usize,
    /// Pictures that would not download or save.
    pub failed: usize,
    /// Why it stopped early, if it did.
    pub stopped: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub stage: Stage,
    /// The game being fetched for.
    pub game: Option<String>,
}

/// Called with each game once a picture of it is saved.
pub type Saved = Arc<dyn Fn(&Game) + Send + Sync>;

/// A Get Artwork run on its own thread.
pub struct Job {
    status: Arc<Mutex<Status>>,
    cancel: Arc<AtomicBool>,
    read: Arc<AtomicU64>,
}

impl Job {
    /// Start fetching for `games` into Shelf's folder `root`.
    pub fn start(root: PathBuf, games: Vec<Game>, net: Arc<dyn Net>) -> Job {
        Job::start_paced(root, games, net, PAUSE)
    }

    /// [`Job::start`], waiting `pause` between pictures.
    pub fn start_paced(root: PathBuf, games: Vec<Game>, net: Arc<dyn Net>, pause: Duration) -> Job {
        Job::start_with(root, games, net, pause, Arc::new(|_: &Game| {}))
    }

    /// [`Job::start_paced`], calling `saved` with each game once a picture
    /// of it is saved, as Shelf does to cut its small copies there and then.
    pub fn start_with(
        root: PathBuf,
        games: Vec<Game>,
        net: Arc<dyn Net>,
        pause: Duration,
        saved: Saved,
    ) -> Job {
        let job = Job {
            status: Arc::new(Mutex::new(Status {
                stage: Stage::Reading { read: 0, of: 0 },
                game: None,
            })),
            cancel: Arc::new(AtomicBool::new(false)),
            read: Arc::new(AtomicU64::new(0)),
        };
        let (status, cancel, read) = (job.status.clone(), job.cancel.clone(), job.read.clone());
        std::thread::spawn(move || {
            let stage = match run(&root, &games, net.as_ref(), &status, &cancel, &read, pause, &*saved) {
                Ok(summary) => Stage::Finished(summary),
                Err(_) if cancel.load(Ordering::Relaxed) => Stage::Finished(Summary {
                    stopped: Some("Stopped".into()),
                    ..Summary::default()
                }),
                Err(e) => Stage::Failed(e.to_string()),
            };
            if let Ok(mut s) = status.lock() {
                s.stage = stage;
                s.game = None;
            }
        });
        job
    }

    pub fn status(&self) -> Status {
        let mut s = self
            .status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(Status {
                stage: Stage::Failed("lost track".into()),
                game: None,
            });
        if let Stage::Reading { of, .. } = s.stage {
            s.stage = Stage::Reading {
                read: self.read.load(Ordering::Relaxed),
                of,
            };
        }
        s
    }

    /// Stop at the next picture, or part way through the list.
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn finished(&self) -> bool {
        matches!(self.status().stage, Stage::Finished(_) | Stage::Failed(_))
    }
}

fn set(status: &Mutex<Status>, stage: Stage, game: Option<String>) {
    if let Ok(mut s) = status.lock() {
        s.stage = stage;
        s.game = game;
    }
}

fn write_if_changed(path: &Path, text: &str) -> io::Result<bool> {
    if std::fs::read_to_string(path).is_ok_and(|t| t == text) {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)?;
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn run(
    root: &Path,
    games: &[Game],
    net: &dyn Net,
    status: &Mutex<Status>,
    cancel: &AtomicBool,
    read: &Arc<AtomicU64>,
    pause: Duration,
    on_saved: &(dyn Fn(&Game) + Send + Sync),
) -> io::Result<Summary> {
    let mut summary = Summary::default();
    let mut index = read_index(root);
    let known = games.iter().all(|g| index.contains_key(g));
    if !(known && index_fresh(root)) {
        let source = net.metadata();
        let (xml, of) = zip::open(source.as_ref(), METADATA_ENTRY, read.clone())?;
        set(status, Stage::Reading { read: 0, of }, None);
        let found = catalog::read(BufReader::with_capacity(1 << 16, xml), games, cancel)?;
        for game in games {
            let entry = match found.get(game) {
                Some(f) => {
                    if write_if_changed(&details_path(root, game), &f.details)? {
                        summary.details += 1;
                    }
                    Some([f.front.clone(), f.back.clone(), f.cart.clone()])
                }
                None => None,
            };
            index.insert(game.clone(), entry);
        }
        write_index(root, &index)?;
    }
    let mut todo = Vec::new();
    for game in games {
        match index.get(game) {
            Some(Some(files)) => {
                for (kind, file) in Kind::ALL.into_iter().zip(files) {
                    if let Some(file) = file {
                        if !kind.on_card(root, game) {
                            todo.push((game, kind, file.clone()));
                        }
                    }
                }
            }
            _ => summary.unknown += 1,
        }
    }
    let of = todo.len();
    for (done, (game, kind, file)) in todo.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            summary.stopped = Some("Stopped".into());
            break;
        }
        if free_space(root).is_some_and(|free| free < SPACE_MARGIN) {
            summary.stopped = Some("The SD card is nearly full".into());
            break;
        }
        set(
            status,
            Stage::Fetching { done, of },
            Some(shelf_ui::game_name(&game.1)),
        );
        match net.picture(&file) {
            Ok(bytes) => {
                let dest = kind.path(root, game);
                let saved = match kind {
                    Kind::Cart => picture::save_cart(&bytes, &dest),
                    _ => picture::save_box(&bytes, &dest),
                };
                match saved {
                    Ok(()) => summary.pictures += 1,
                    Err(_) => summary.failed += 1,
                }
                if saved.is_ok() {
                    on_saved(game);
                }
            }
            Err(NetError::Busy) => {
                summary.stopped = Some("LaunchBox is busy: try again later".into());
                break;
            }
            Err(NetError::Missing(_)) => summary.failed += 1,
            Err(NetError::Down(e)) => {
                summary.stopped = Some(format!("The connection was lost: {e}"));
                break;
            }
        }
        std::thread::sleep(pause);
    }
    Ok(summary)
}
