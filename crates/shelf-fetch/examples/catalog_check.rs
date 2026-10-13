//! Reads a local Metadata.zip for the games in a folder of Top 100 lists and
//! prints how many matched, and any details differing from DIR's files.
//!
//! cargo run --release -p shelf-fetch --example catalog_check METADATA_ZIP LISTS_DIR DETAILS_DIR
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (zip, lists, details) = (&args[1], &args[2], &args[3]);
    let mut games = Vec::new();
    for entry in std::fs::read_dir(lists).unwrap().flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
        for line in text.lines() {
            let p = std::path::Path::new(line.trim());
            let (Some(dir), Some(stem)) = (p.parent().and_then(|d| d.file_name()), p.file_stem()) else {
                continue;
            };
            if let Some(platform) = shelf_store::system_platform(&dir.to_string_lossy()) {
                games.push((platform, stem.to_string_lossy().into_owned()));
            }
        }
    }
    let t = std::time::Instant::now();
    let source = shelf_fetch::zip::FileSource(zip.into());
    let (xml, of) = shelf_fetch::zip::open(&source, "Metadata.xml", Arc::new(AtomicU64::new(0))).unwrap();
    let found = shelf_fetch::catalog::read(std::io::BufReader::new(xml), &games, &AtomicBool::new(false)).unwrap();
    println!("{} games, {} found, {} compressed bytes, {:?}", games.len(), found.len(), of, t.elapsed());
    let (mut same, mut differ, mut missing) = (0, 0, 0);
    for g in &games {
        let path = std::path::Path::new(details).join(g.0.dir_name()).join(format!("{}.txt", g.1));
        let old = std::fs::read_to_string(&path).ok();
        match (old, found.get(g)) {
            (Some(o), Some(f)) if o == f.details => same += 1,
            (Some(o), Some(f)) => {
                differ += 1;
                if differ <= 3 {
                    println!("DIFF {:?}\n--- py\n{o}--- rs\n{}", g, f.details);
                }
            }
            (Some(_), None) => { missing += 1; println!("MISSING {g:?}"); }
            (None, Some(_)) => println!("EXTRA {g:?}"),
            (None, None) => {}
        }
    }
    let with = |k: fn(&shelf_fetch::Found) -> bool| found.values().filter(|f| k(f)).count();
    println!("same {same}, differ {differ}, missing {missing}; fronts {} backs {} carts {}",
        with(|f| f.front.is_some()), with(|f| f.back.is_some()), with(|f| f.cart.is_some()));
}
