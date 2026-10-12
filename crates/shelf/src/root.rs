use std::path::Path;

/// The folders slot keeps on the card: its settings, and its caches.
pub const DIRS: [&str; 2] = ["Config", "System"];

pub fn ensure(root: &Path) {
    for sub in DIRS {
        let _ = std::fs::create_dir_all(root.join(sub));
    }
    if let Err(e) = shelf_store::move_config(root) {
        eprintln!("slot: config: {e}");
    }
}
