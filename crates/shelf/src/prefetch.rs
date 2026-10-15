use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

use shelf_input::Millis;

pub const PREFETCH_REST_MS: Millis = 500;

/// How much of a rom is read on the first rest: all of nearly every
/// console's games, and the start of a large GBA one. Reading a whole
/// 32 MB rom for each cart paused on kept the card busy, for a launch
/// that may not come.
pub const PREFETCH_HEAD: u64 = 8 << 20;

/// How long a cart is rested on before the rest of a large rom is read
/// too: by then it is likely to be played.
pub const PREFETCH_WHOLE_MS: Millis = 3_000;

const CHUNK: usize = 1 << 20;

#[derive(Default)]
pub struct Prefetch {
    resting: Option<(PathBuf, Millis)>,
    read: Option<PathBuf>,
    /// Whether the rest of the rom past the head has been asked for.
    whole: bool,
    turn: Arc<AtomicU64>,
    worker: Option<JoinHandle<u64>>,
}

impl Prefetch {
    pub fn browse(&mut self, rom: Option<&Path>, now: Millis) {
        let Some(rom) = rom else {
            self.resting = None;
            self.read = None;
            self.whole = false;
            self.turn.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let since = match &self.resting {
            Some((at, since)) if at == rom => *since,
            _ => {
                self.resting = Some((rom.to_path_buf(), now));
                self.read = None;
                self.whole = false;
                self.turn.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };
        let rested = now.saturating_sub(since);
        let most = if self.read.as_deref() != Some(rom) {
            if rested < PREFETCH_REST_MS {
                return;
            }
            self.read = Some(rom.to_path_buf());
            PREFETCH_HEAD
        } else if !self.whole && rested >= PREFETCH_WHOLE_MS {
            self.whole = true;
            let len = std::fs::metadata(rom).map_or(0, |m| m.len());
            if len <= PREFETCH_HEAD {
                return;
            }
            // From the start again: the head is in memory by now, and
            // a first read cut short leaves no gap.
            u64::MAX
        } else {
            return;
        };
        let turn = self.turn.clone();
        let mine = turn.fetch_add(1, Ordering::Relaxed) + 1;
        let path = rom.to_path_buf();
        self.worker = std::thread::Builder::new()
            .name("shelf-prefetch".into())
            .spawn(move || read_through(&path, &turn, mine, most))
            .ok();
    }

    pub fn reading(&self) -> Option<&Path> {
        self.read.as_deref()
    }

    pub fn wait(&mut self) -> Option<u64> {
        self.worker.take().and_then(|w| w.join().ok())
    }
}

/// Read at most `most` bytes of `path`, while the turn is still `mine`:
/// the bytes read.
fn read_through(path: &Path, turn: &AtomicU64, mine: u64, most: u64) -> u64 {
    let start = Instant::now();
    let Ok(mut file) = std::fs::File::open(path) else {
        return 0;
    };
    let mut buf = vec![0u8; CHUNK];
    let mut total = 0u64;
    while turn.load(Ordering::Relaxed) == mine && total < most {
        let want = (most - total).min(CHUNK as u64) as usize;
        match file.read(&mut buf[..want]) {
            Ok(0) | Err(_) => break,
            Ok(n) => total += n as u64,
        }
    }
    eprintln!(
        "shelf: prefetch: {} bytes of {} in {:.1} ms",
        total,
        path.display(),
        start.elapsed().as_secs_f64() * 1e3
    );
    total
}
