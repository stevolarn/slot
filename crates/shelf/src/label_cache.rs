use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::UNIX_EPOCH;

use shelf_store::Cart;
use shelf_ui::{
    cart_box, cart_photo_face, label_art as cut_label, label_from_box, label_size, CartFace,
};

const DIR: &str = "System/Cache/Labels";
const HEAD: usize = 24;
const CARTS: &str = "Carts";

pub fn label_art(root: &Path, cart: &Cart) -> Option<Vec<u8>> {
    let src = cart.label.as_deref()?;
    let (w, h) = label_size(cart);
    let stamp = stamp(src)?;
    let cached = cache_path(root, cart);
    if let Some(art) = read(&cached, stamp, w, h) {
        return art;
    }
    let art = cut_label(cart);
    write(&cached, stamp, w, h, art.as_deref());
    art
}

/// The photo of a game's real cart, kept at `Carts/<console>/<rom name>.png`
/// under the root, fitted to the cart's size and drawn as its whole face.
pub fn cart_photo(root: &Path, cart: &Cart) -> Option<CartFace> {
    photo_face(root, cart, photo_path(root, cart))
}

/// [`cart_photo`], told by a listing of the `Carts` folders which games have
/// a photo, so a game without one costs no look at the card.
pub fn cart_photo_in(root: &Path, cart: &Cart, photos: &Photos) -> Option<CartFace> {
    photo_face(root, cart, photos.path_of(root, cart)?)
}

/// The photos in the card's `Carts/<console>` folders, listed once.
pub struct Photos {
    listed: HashMap<shelf_store::Platform, HashMap<String, String>>,
}

impl Photos {
    pub fn list(root: &Path) -> Photos {
        Photos {
            listed: shelf_store::Platform::ALL
                .into_iter()
                .map(|p| {
                    (
                        p,
                        shelf_store::listing(&root.join(CARTS).join(p.dir_name())),
                    )
                })
                .filter(|(_, names)| !names.is_empty())
                .collect(),
        }
    }

    fn path_of(&self, root: &Path, cart: &Cart) -> Option<PathBuf> {
        let name = self
            .listed
            .get(&cart.platform)?
            .get(&format!("{}.png", cart.stem).to_lowercase())?;
        Some(root.join(CARTS).join(cart.platform.dir_name()).join(name))
    }
}

fn photo_face(root: &Path, cart: &Cart, src: PathBuf) -> Option<CartFace> {
    let stamp = stamp(&src)?;
    let (w, h) = cart_box(cart.platform);
    let cached = root
        .join(DIR)
        .join(cart.platform.dir_name())
        .join(format!("{}.{}.rgba", cart.stem, photo_cut(cart)));
    let rgba = match read(&cached, stamp, w, h) {
        Some(rgba) => rgba,
        None => {
            let rgba = cart_photo_face(cart, &src).map(|f| f.rgba);
            write(&cached, stamp, w, h, rgba.as_deref());
            rgba
        }
    };
    Some(CartFace { rgba: rgba?, w, h })
}

/// The name a cut photo is kept under: the Colour carts', made see-through
/// where the photo was black, under their own so older cuts are not used.
fn photo_cut(cart: &Cart) -> &'static str {
    match cart.platform {
        shelf_store::Platform::Gbc => "clear",
        _ => "cart",
    }
}

pub fn photo_path(root: &Path, cart: &Cart) -> PathBuf {
    root.join(CARTS)
        .join(cart.platform.dir_name())
        .join(format!("{}.png", cart.stem))
}

fn write(cached: &Path, stamp: (u64, u64), w: u32, h: u32, art: Option<&[u8]>) {
    if let Some(dir) = cached.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let (cw, ch) = if art.is_some() { (w, h) } else { (0, 0) };
    let mut bytes = Vec::with_capacity(HEAD + art.map_or(0, <[u8]>::len));
    bytes.extend_from_slice(&stamp.0.to_le_bytes());
    bytes.extend_from_slice(&stamp.1.to_le_bytes());
    bytes.extend_from_slice(&cw.to_le_bytes());
    bytes.extend_from_slice(&ch.to_le_bytes());
    bytes.extend_from_slice(art.unwrap_or_default());
    if let Err(e) = std::fs::write(cached, &bytes) {
        eprintln!("shelf: label cache: {}: {e}", cached.display());
    }
}

const BOX_DIR: &str = "System/Cache/Boxes";
const BOX_HEAD: usize = 24;

/// A size box pictures are cut to and kept at on the card: its name in
/// the file's, the most it is, and how a picture is cut to it.
struct BoxCut {
    name: &'static str,
    w: u32,
    h: u32,
    make: fn(&Path) -> Option<CartFace>,
}

/// The store shelf's boxes.
const STORE_CUT: BoxCut = BoxCut {
    name: "box",
    w: shelf_ui::STORE_BOX_W,
    h: shelf_ui::STORE_BOX_H,
    make: shelf_ui::store_box_face,
};

/// The box shelf's, larger.
const TALL_CUT: BoxCut = BoxCut {
    name: "tall",
    w: shelf_ui::TALL_BOX_W,
    h: shelf_ui::TALL_BOX_H,
    make: shelf_ui::tall_box_face,
};

/// A game's box picture shrunk to the size the store shelf draws it at,
/// kept on the card beside the labels so it is only ever cut once.
pub fn box_thumb(root: &Path, cart: &Cart, src: &Path) -> Option<CartFace> {
    cut_box(root, cart, src, &STORE_CUT)
}

/// The same for the box shelf, at its larger size.
pub fn tall_box_thumb(root: &Path, cart: &Cart, src: &Path) -> Option<CartFace> {
    cut_box(root, cart, src, &TALL_CUT)
}

fn cut_box(root: &Path, cart: &Cart, src: &Path, cut: &BoxCut) -> Option<CartFace> {
    let stamp = stamp(src)?;
    let cached = root
        .join(BOX_DIR)
        .join(cart.platform.dir_name())
        .join(format!("{}.{}.rgba", cart.stem, cut.name));
    if let Some(face) = read_box(&cached, stamp, cut) {
        return Some(face);
    }
    let face = (cut.make)(src)?;
    write_box(&cached, stamp, &face);
    Some(face)
}

fn read_box(path: &Path, stamp: (u64, u64), cut: &BoxCut) -> Option<CartFace> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut head = [0u8; BOX_HEAD];
    f.read_exact(&mut head).ok()?;
    let word = |at: usize| u64::from_le_bytes(head[at..at + 8].try_into().unwrap_or_default());
    let half = |at: usize| u32::from_le_bytes(head[at..at + 4].try_into().unwrap_or_default());
    let (w, h) = (half(16), half(20));
    let fits = w >= 1 && h >= 1 && w <= cut.w && h <= cut.h && (w == cut.w || h == cut.h);
    if (word(0), word(8)) != stamp || !fits {
        return None;
    }
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    f.read_exact(&mut rgba).ok()?;
    let mut extra = [0u8; 1];
    if f.read(&mut extra).ok()? != 0 {
        return None;
    }
    Some(CartFace { rgba, w, h })
}

fn write_box(cached: &Path, stamp: (u64, u64), face: &CartFace) {
    if let Some(dir) = cached.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut bytes = Vec::with_capacity(BOX_HEAD + face.rgba.len());
    bytes.extend_from_slice(&stamp.0.to_le_bytes());
    bytes.extend_from_slice(&stamp.1.to_le_bytes());
    bytes.extend_from_slice(&face.w.to_le_bytes());
    bytes.extend_from_slice(&face.h.to_le_bytes());
    bytes.extend_from_slice(&face.rgba);
    if let Err(e) = std::fs::write(cached, &bytes) {
        eprintln!("shelf: box cache: {}: {e}", cached.display());
    }
}

pub fn fresh(root: &Path, cart: &Cart) -> bool {
    let Some(src) = cart.label.as_deref() else {
        return true;
    };
    let Some(stamp) = stamp(src) else {
        return true;
    };
    let (w, h) = label_size(cart);
    std::fs::File::open(cache_path(root, cart))
        .and_then(|mut f| {
            let mut head = [0u8; HEAD];
            f.read_exact(&mut head)?;
            Ok((head, f.metadata()?.len()))
        })
        .is_ok_and(|(head, len)| {
            body_len(&head, stamp, w, h).is_some_and(|body| len == (HEAD + body) as u64)
        })
}

pub fn stale<'a>(root: &Path, carts: impl Iterator<Item = &'a Cart>) -> Vec<Cart> {
    let mut cached = HashMap::new();
    carts
        .filter(|c| c.label.is_some())
        .filter(|c| {
            !cached
                .entry(c.platform)
                .or_insert_with(|| {
                    shelf_store::listing(&root.join(DIR).join(c.platform.dir_name()))
                })
                .contains_key(&cache_name(c).to_lowercase())
        })
        .cloned()
        .collect()
}

pub fn cache_path(root: &Path, cart: &Cart) -> PathBuf {
    root.join(DIR)
        .join(cart.platform.dir_name())
        .join(cache_name(cart))
}

fn cache_name(cart: &Cart) -> String {
    if label_from_box(cart) {
        format!("{}.top.rgba", cart.stem)
    } else {
        format!("{}.rgba", cart.stem)
    }
}

pub struct Rebuild {
    done: Arc<AtomicUsize>,
    total: usize,
}

impl Rebuild {
    pub fn start(root: &Path, carts: Vec<Cart>) -> Self {
        let total = carts.len();
        let done = Arc::new(AtomicUsize::new(0));
        let next = Arc::new(AtomicUsize::new(0));
        let carts = Arc::new(carts);
        let threads = thread::available_parallelism().map_or(1, |n| n.get());
        for n in 0..threads.min(total) {
            let (root, carts, next, done) = (
                root.to_path_buf(),
                carts.clone(),
                next.clone(),
                done.clone(),
            );
            let spawned = thread::Builder::new()
                .name(format!("shelf-labels-{n}"))
                .spawn(move || loop {
                    let Some(cart) = carts.get(next.fetch_add(1, Ordering::Relaxed)) else {
                        return;
                    };
                    let _ = label_art(&root, cart);
                    done.fetch_add(1, Ordering::Release);
                });
            if let Err(e) = spawned {
                eprintln!("shelf: label cache: rebuild thread failed to start: {e}");
            }
        }
        Rebuild { done, total }
    }

    pub fn done(&self) -> usize {
        self.done.load(Ordering::Acquire).min(self.total)
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn finished(&self) -> bool {
        self.done() == self.total
    }
}

fn stamp(src: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(src).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    Some((meta.len(), mtime))
}

fn read(path: &Path, stamp: (u64, u64), w: u32, h: u32) -> Option<Option<Vec<u8>>> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut head = [0u8; HEAD];
    f.read_exact(&mut head).ok()?;
    let n = body_len(&head, stamp, w, h)?;
    // The picture is read straight into its own buffer, so it is not copied
    // out of a read of the whole file.
    let mut rgba = vec![0u8; n];
    f.read_exact(&mut rgba).ok()?;
    let mut extra = [0u8; 1];
    if f.read(&mut extra).ok()? != 0 {
        return None;
    }
    Some((n != 0).then_some(rgba))
}

fn body_len(head: &[u8], stamp: (u64, u64), w: u32, h: u32) -> Option<usize> {
    let word = |at: usize| u64::from_le_bytes(head[at..at + 8].try_into().unwrap_or_default());
    let half = |at: usize| u32::from_le_bytes(head[at..at + 4].try_into().unwrap_or_default());
    if (word(0), word(8)) != stamp {
        return None;
    }
    match (half(16), half(20)) {
        (0, 0) => Some(0),
        size if size == (w, h) => Some((w * h * 4) as usize),
        _ => None,
    }
}
