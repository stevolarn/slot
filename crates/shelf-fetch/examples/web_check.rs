//! A real Get Artwork run against LaunchBox for two games, into OUT.
//!
//! cargo run --release -p shelf-fetch --example web_check OUT
use std::sync::Arc;

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("OUT"));
    let games = vec![
        (shelf_store::Platform::Md, "Streets of Rage 3 (USA)".to_string()),
        (shelf_store::Platform::Gba, "Legend of Zelda, The - The Minish Cap (Europe)".to_string()),
    ];
    let t = std::time::Instant::now();
    let job = shelf_fetch::Job::start(out, games, Arc::new(shelf_fetch::Web::default()));
    let mut last = String::new();
    loop {
        let s = job.status();
        let line = match &s.stage {
            shelf_fetch::Stage::Reading { read, of } => format!("reading {}%", read * 100 / (*of).max(1)),
            other => format!("{other:?} {:?}", s.game),
        };
        if line != last {
            println!("{:>5.1}s {line}", t.elapsed().as_secs_f32());
            last = line;
        }
        if job.finished() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
