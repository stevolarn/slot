use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;

use shelf_gfx::{Compositor, TexId};
use shelf_store::Cart;
use shelf_ui::{cart_face_with, CartFace};

use crate::app::{box_art_of, box_front_of, App, FaceKind, FaceWant};
use crate::label_cache::{box_thumb, cart_photo_in, label_art, tall_box_thumb, Photos};

const WORKERS: usize = 3;

/// Which picture of which game on which shelf.
type Key = (usize, usize, FaceKind);

#[derive(Default)]
struct Queue {
    jobs: VecDeque<(Key, Cart)>,
    closed: bool,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

/// The card's cart photos, listed by whichever worker needs them first.
type PhotoList = Arc<OnceLock<Photos>>;

pub struct CartFaces {
    queue: Shared,
    built: Receiver<(Key, CartFace)>,
    /// The pictures asked for and not placed yet, with the name of the
    /// game each was asked for: a picture that comes back for a different
    /// game, the shelf having been put in another order, is thrown away.
    pending: HashMap<Key, String>,
    asked: Vec<FaceWant>,
    free: Vec<TexId>,
    /// The shelves' changes the pictures asked for were last known to be
    /// for.
    epoch: u32,
}

impl CartFaces {
    pub fn spawn(root: PathBuf) -> Self {
        let queue: Shared = Arc::default();
        let (outbox, built) = mpsc::channel();
        let photos: PhotoList = Arc::default();
        for n in 0..WORKERS {
            let (queue, outbox, root) = (queue.clone(), outbox.clone(), root.clone());
            let photos = photos.clone();
            let spawned = thread::Builder::new()
                .name(format!("shelf-carts-{n}"))
                .spawn(move || work(&queue, &outbox, &root, &photos));
            if let Err(e) = spawned {
                eprintln!("shelf: cart faces: worker thread failed to start: {e}");
            }
        }
        CartFaces {
            queue,
            built,
            pending: HashMap::new(),
            asked: Vec::new(),
            free: Vec::new(),
            epoch: 0,
        }
    }

    /// Upload the faces built since the last call and ask for the ones now
    /// wanted. With `wait`, as at boot, block until every cart on screen
    /// has its face; otherwise a cart whose face is not built yet is drawn
    /// plain for a frame or two rather than holding the frame up.
    pub fn sync(&mut self, app: &mut App, compositor: &mut Compositor, wait: bool) {
        if app.face_epoch() != self.epoch {
            // The shelves were put in another order, or shown another way:
            // what was asked for is for places that now hold other games.
            self.epoch = app.face_epoch();
            let (lock, _) = &*self.queue;
            lock.lock().unwrap_or_else(|e| e.into_inner()).jobs.clear();
            self.pending.clear();
            self.asked.clear();
        }
        self.free.extend(app.shed_faces());
        while let Ok((key, face)) = self.built.try_recv() {
            self.place(app, compositor, key, face);
        }
        let wants = app.face_wants();
        if wants != self.asked {
            self.requeue(app, &wants);
            self.asked = wants;
        }
        if !wait {
            return;
        }
        let mut waiting: Vec<Key> = self
            .asked
            .iter()
            .filter(|w| w.on_screen)
            .map(|w| (w.shelf, w.index, w.kind))
            .collect();
        while !waiting.is_empty() {
            let Ok((key, face)) = self.built.recv() else {
                return;
            };
            waiting.retain(|k| *k != key);
            self.place(app, compositor, key, face);
        }
    }

    /// Whether faces are still being built.
    pub fn busy(&self) -> bool {
        !self.pending.is_empty()
    }

    fn requeue(&mut self, app: &App, wants: &[FaceWant]) {
        let (lock, wake) = &*self.queue;
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        for (key, _) in queue.jobs.drain(..) {
            self.pending.remove(&key);
        }
        for w in wants {
            let key = (w.shelf, w.index, w.kind);
            if self.pending.contains_key(&key) {
                continue;
            }
            if let Some(cart) = app.face_cart(w.shelf, w.index) {
                self.pending.insert(key, cart.stem.clone());
                queue.jobs.push_back((key, cart.clone()));
            }
        }
        wake.notify_all();
    }

    fn place(&mut self, app: &mut App, compositor: &mut Compositor, key: Key, face: CartFace) {
        let asked_for = self.pending.remove(&key);
        self.asked.retain(|w| (w.shelf, w.index, w.kind) != key);
        // Only a picture of the game the place still holds goes up.
        let still = app.face_cart(key.0, key.1).map(|c| &c.stem);
        if asked_for.as_ref() != still {
            return;
        }
        let tex = match self.free.pop() {
            Some(tex) => {
                compositor.update_texture(tex, face.w, face.h, &face.rgba);
                tex
            }
            None => compositor.create_texture(face.w, face.h, &face.rgba),
        };
        let old = match key.2 {
            FaceKind::Cart => app.set_cart_face(key.0, key.1, tex),
            FaceKind::Box => app.set_store_face(key.0, key.1, tex, face.w, face.h),
            FaceKind::Tall => app.set_tall_face(key.0, key.1, tex, face.w, face.h),
        };
        self.free.extend(old);
    }
}

impl Drop for CartFaces {
    fn drop(&mut self) {
        let (lock, wake) = &*self.queue;
        lock.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        wake.notify_all();
    }
}

fn work(queue: &Shared, outbox: &Sender<(Key, CartFace)>, root: &Path, photos: &PhotoList) {
    let (lock, wake) = &**queue;
    loop {
        let (key, cart) = {
            let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if q.closed {
                    return;
                }
                if let Some(job) = q.jobs.pop_front() {
                    break job;
                }
                q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        let face = match key.2 {
            FaceKind::Cart => {
                let photos = photos.get_or_init(|| Photos::list(root));
                cart_photo_in(root, &cart, photos)
                    .unwrap_or_else(|| cart_face_with(&cart, label_art(root, &cart)))
            }
            // A box from its picture, fitted small and kept on the card, or
            // a plain one with the game's name when it has no picture.
            FaceKind::Box => box_art_of(&cart)
                .and_then(|path| box_thumb(root, &cart, &path))
                .unwrap_or_else(|| shelf_ui::plain_box_face(&cart)),
            // The box shelf draws its box large: from a full-size scan of
            // the front when there is one.
            FaceKind::Tall => box_front_of(root, &cart)
                .or_else(|| box_art_of(&cart))
                .and_then(|path| tall_box_thumb(root, &cart, &path))
                .unwrap_or_else(|| shelf_ui::plain_box_face(&cart)),
        };
        if outbox.send((key, face)).is_err() {
            return;
        }
    }
}
