use std::path::{Path, PathBuf};

/// A seed for the wallpaper shuffle that differs from one boot to the next.
pub fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The wallpaper to show now, with the shuffled deck written down at once.
pub fn pick(root: &Path, seed: u64) -> Option<PathBuf> {
    let chosen = choose(root, seed)?;
    chosen.save();
    Some(chosen.path)
}

/// A wallpaper chosen, and the deck that records the choice, not yet
/// written: writing it syncs the card, which a boot should not wait for.
pub struct Chosen {
    pub path: PathBuf,
    deck: PathBuf,
    text: String,
}

impl Chosen {
    pub fn save(&self) {
        if let Err(e) = shelf_store::atomic_write(&self.deck, self.text.as_bytes()) {
            eprintln!("shelf: wallpaper.deck: {e}");
        }
    }
}

/// Pick the wallpaper and decode it on a thread of its own, so the boot
/// does not wait for the picture; the deck is written after the picture is
/// handed over. The receiver gets the dimmed picture, or None when there is
/// no wallpaper or it would not decode.
pub fn load_in_background(
    root: PathBuf,
    seed: u64,
) -> Option<std::sync::mpsc::Receiver<Option<Vec<u8>>>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("shelf-wallpaper".into())
        .spawn(move || {
            let chosen = choose(&root, seed);
            let face = chosen
                .as_ref()
                .and_then(|c| shelf_ui::wallpaper_face(&c.path));
            let _ = tx.send(face);
            if let Some(chosen) = chosen {
                chosen.save();
            }
        })
        .ok()?;
    Some(rx)
}

pub fn choose(root: &Path, seed: u64) -> Option<Chosen> {
    let dir = root.join("Wallpapers");
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| !shelf_store::is_hidden(p))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .filter_map(|p| Some(p.file_name()?.to_str()?.to_owned()))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    let mut rng = Rng(seed);
    let path = root.join("Config").join("wallpaper.deck");
    let (mut shown, mut queued) = read_deck(&path);
    shown.retain(|n| files.contains(n));
    queued.retain(|n| files.contains(n));
    for n in &files {
        if !shown.contains(n) && !queued.contains(n) {
            let at = rng.below(queued.len() + 1);
            queued.insert(at, n.clone());
        }
    }
    if queued.is_empty() {
        let last = shown.pop();
        queued = files;
        for i in (1..queued.len()).rev() {
            queued.swap(i, rng.below(i + 1));
        }
        if queued.len() > 1 && last.as_ref() == queued.first() {
            let end = queued.len() - 1;
            queued.swap(0, end);
        }
        shown.clear();
    }
    let next = queued.remove(0);
    shown.push(next.clone());
    Some(Chosen {
        path: dir.join(next),
        deck: path,
        text: write_deck(&shown, &queued),
    })
}

fn read_deck(path: &Path) -> (Vec<String>, Vec<String>) {
    let Some(text) = std::fs::read(path)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
    else {
        return (Vec::new(), Vec::new());
    };
    let mut lines = text.lines();
    let Some(shown) = lines.next().and_then(|l| l.parse::<usize>().ok()) else {
        return (Vec::new(), Vec::new());
    };
    let mut order: Vec<String> = lines.map(str::to_owned).collect();
    let queued = order.split_off(shown.min(order.len()));
    (order, queued)
}

fn write_deck(shown: &[String], queued: &[String]) -> String {
    let mut text = format!("{}\n", shown.len());
    for n in shown.iter().chain(queued) {
        text.push_str(n);
        text.push('\n');
    }
    text
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        (z % n as u64) as usize
    }
}
