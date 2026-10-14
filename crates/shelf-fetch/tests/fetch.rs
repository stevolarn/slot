use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shelf_fetch::zip::{FileSource, Source};
use shelf_fetch::{Job, Kind, Net, NetError, Stage};
use shelf_store::Platform;

const XML: &str = r#"<?xml version="1.0" standalone="yes"?>
<LaunchBox>
  <Game>
    <Name>Streets of Rage 3</Name>
    <ReleaseDate>1994-03-17</ReleaseDate>
    <Overview>Mr. X &amp; the Syndicate are back.</Overview>
    <DatabaseID>10</DatabaseID>
    <Platform>Sega Genesis</Platform>
    <CommunityRating>4.2</CommunityRating>
    <CommunityRatingCount>360</CommunityRatingCount>
    <Genres>Beat 'em Up;Action</Genres>
    <MaxPlayers>2</MaxPlayers>
    <Cooperative>true</Cooperative>
    <Publisher>Sega</Publisher>
  </Game>
  <Game>
    <Name>Streets of Rage 3</Name>
    <DatabaseID>11</DatabaseID>
    <Platform>Sega Genesis</Platform>
    <CommunityRatingCount>3</CommunityRatingCount>
  </Game>
  <Game>
    <Name>Streets of Rage 3</Name>
    <DatabaseID>12</DatabaseID>
    <Platform>Sega Saturn</Platform>
    <CommunityRatingCount>900</CommunityRatingCount>
  </Game>
  <Game>
    <Name>Kirby's Dream Land</Name>
    <ReleaseYear>1992</ReleaseYear>
    <DatabaseID>20</DatabaseID>
    <Platform>Nintendo Game Boy</Platform>
  </Game>
  <GameAlternateName>
    <AlternateName>Hoshi no Kirby</AlternateName>
    <DatabaseID>20</DatabaseID>
  </GameAlternateName>
  <GameImage>
    <DatabaseID>10</DatabaseID>
    <FileName>sor3-jp.png</FileName>
    <Type>Box - Front</Type>
    <Region>Japan</Region>
  </GameImage>
  <GameImage>
    <DatabaseID>10</DatabaseID>
    <FileName>sor3-eu.png</FileName>
    <Type>Box - Front</Type>
    <Region>Europe</Region>
  </GameImage>
  <GameImage>
    <DatabaseID>10</DatabaseID>
    <FileName>sor3-back.png</FileName>
    <Type>Box - Back</Type>
  </GameImage>
  <GameImage>
    <DatabaseID>10</DatabaseID>
    <FileName>sor3-cart-fan.png</FileName>
    <Type>Fanart - Cart - Front</Type>
    <Region>Europe</Region>
  </GameImage>
  <GameImage>
    <DatabaseID>10</DatabaseID>
    <FileName>sor3-cart.png</FileName>
    <Type>Cart - Front</Type>
    <Region>Japan</Region>
  </GameImage>
  <GameImage>
    <DatabaseID>10</DatabaseID>
    <FileName>sor3-title.png</FileName>
    <Type>Screenshot - Game Title</Type>
  </GameImage>
  <GameImage>
    <DatabaseID>10</DatabaseID>
    <FileName>sor3-play.png</FileName>
    <Type>Screenshot - Gameplay</Type>
  </GameImage>
  <GameImage>
    <DatabaseID>11</DatabaseID>
    <FileName>other.png</FileName>
    <Type>Box - Front</Type>
  </GameImage>
  <GameImage>
    <DatabaseID>20</DatabaseID>
    <FileName>kirby-front.png</FileName>
    <Type>Box - Front</Type>
  </GameImage>
</LaunchBox>
"#;

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (!(crc & 1)).wrapping_add(1));
        }
    }
    !crc
}

/// A zip holding `files`, each deflated, as a zip program writes it.
fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut dir = Vec::new();
    for (name, data) in files {
        let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(data).unwrap();
        let packed = enc.finish().unwrap();
        let at = out.len() as u32;
        let crc = crc32(data);
        // A local header with an extra field the directory does not have.
        out.extend(0x0403_4b50u32.to_le_bytes());
        out.extend([20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
        out.extend(crc.to_le_bytes());
        out.extend((packed.len() as u32).to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(4u16.to_le_bytes());
        out.extend(name.as_bytes());
        out.extend([0xfe, 0xca, 0, 0]);
        out.extend(&packed);
        dir.extend(0x0201_4b50u32.to_le_bytes());
        dir.extend([20, 0, 20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
        dir.extend(crc.to_le_bytes());
        dir.extend((packed.len() as u32).to_le_bytes());
        dir.extend((data.len() as u32).to_le_bytes());
        dir.extend((name.len() as u16).to_le_bytes());
        dir.extend([0u8; 12]);
        dir.extend(at.to_le_bytes());
        dir.extend(name.as_bytes());
    }
    let dir_at = out.len() as u32;
    let n = files.len() as u16;
    out.extend(&dir);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0, 0, 0, 0]);
    out.extend(n.to_le_bytes());
    out.extend(n.to_le_bytes());
    out.extend((dir.len() as u32).to_le_bytes());
    out.extend(dir_at.to_le_bytes());
    out.extend([0, 0]);
    out
}

/// A `w` by `h` PNG: opaque in the middle, see-through for `border` pixels
/// round the edge.
fn png_of(w: u32, h: u32, border: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in border..h - border {
        for x in border..w - border {
            let i = ((y * w + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[200, 40, 40, 255]);
        }
    }
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
    out
}

struct FakeNet {
    zip: PathBuf,
    metadata_reads: AtomicUsize,
    asked: Mutex<Vec<String>>,
    busy: Option<&'static str>,
}

impl Net for FakeNet {
    fn metadata(&self) -> Box<dyn Source + Send> {
        self.metadata_reads.fetch_add(1, Ordering::Relaxed);
        Box::new(FileSource(self.zip.clone()))
    }

    fn picture(&self, file: &str) -> Result<Vec<u8>, NetError> {
        self.asked.lock().unwrap().push(file.to_string());
        if Some(file) == self.busy {
            return Err(NetError::Busy);
        }
        Ok(match file {
            "sor3-cart.png" => png_of(800, 600, 100),
            "sor3-title.png" | "sor3-play.png" => png_of(320, 224, 0),
            _ => png_of(1000, 1400, 0),
        })
    }
}

fn setup(busy: Option<&'static str>) -> (tempfile::TempDir, Arc<FakeNet>, Vec<shelf_fetch::Game>) {
    let d = tempfile::tempdir().unwrap();
    let zip = d.path().join("Metadata.zip");
    std::fs::write(
        &zip,
        zip_of(&[("Platforms.xml", b"<LaunchBox/>"), ("Metadata.xml", XML.as_bytes())]),
    )
    .unwrap();
    let net = Arc::new(FakeNet {
        zip,
        metadata_reads: AtomicUsize::new(0),
        asked: Mutex::new(Vec::new()),
        busy,
    });
    let games = vec![
        (Platform::Md, "Streets of Rage 3 (USA)".to_string()),
        (Platform::Gbc, "Hoshi no Kirby (Japan)".to_string()),
        (Platform::Snes, "Not In LaunchBox (USA)".to_string()),
    ];
    (d, net, games)
}

fn run(root: &Path, net: &Arc<FakeNet>, games: &[shelf_fetch::Game]) -> shelf_fetch::Summary {
    let job = Job::start_paced(root.to_path_buf(), games.to_vec(), net.clone(), Duration::ZERO);
    for _ in 0..500 {
        match job.status().stage {
            Stage::Finished(s) => return s,
            Stage::Failed(e) => panic!("the run failed: {e}"),
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    panic!("the run did not finish");
}

fn size_of(path: &Path) -> (u32, u32) {
    let (_, w, h) = shelf_ui::decode_bytes(&std::fs::read(path).unwrap()).unwrap();
    (w, h)
}

#[test]
fn a_run_writes_details_and_pictures_for_the_games_launchbox_has() {
    let (d, net, games) = setup(None);
    let root = d.path().join("Shelf");
    let summary = run(&root, &net, &games);
    assert_eq!(summary.unknown, 1, "{summary:?}");
    assert_eq!(summary.details, 2);
    assert_eq!(summary.pictures, 6);
    assert_eq!(summary.stopped, None);

    // The details of the most voted Mega Drive entry, not the Saturn one.
    let details = std::fs::read_to_string(root.join("Details/MD/Streets of Rage 3 (USA).txt")).unwrap();
    assert_eq!(
        details,
        "name=Streets of Rage 3\nyear=1994\ngenre=Beat 'em Up,Action\npublisher=Sega\nplayers=2\n\
         coop=yes\nrating=4.20\nvotes=360\noverview=Mr. X & the Syndicate are back.\n"
    );
    // Found under Game Boy by its other name.
    assert!(root.join("Details/GBC/Hoshi no Kirby (Japan).txt").is_file());

    // The European front, the photo rather than the fan art, at Shelf's sizes.
    let asked = net.asked.lock().unwrap().clone();
    assert!(asked.contains(&"sor3-eu.png".to_string()), "{asked:?}");
    assert!(asked.contains(&"sor3-cart.png".to_string()), "{asked:?}");
    let front = root.join("Boxes/MD/Streets of Rage 3 (USA).front.jpg");
    assert!(std::fs::read(&front).unwrap().starts_with(&[0xff, 0xd8]), "not a JPEG");
    assert_eq!(size_of(&front), (429, 600));
    // The cart trimmed to its opaque part, 600 by 400, then fitted.
    let cart = root.join("Carts/MD/Streets of Rage 3 (USA).png");
    assert_eq!(size_of(&cart), (360, 240));
    assert!(root.join("Boxes/GBC/Hoshi no Kirby (Japan).front.jpg").is_file());
    assert!(!root.join("Boxes/GBC/Hoshi no Kirby (Japan).back.jpg").exists());
    // The title screen and a gameplay screenshot, kept at their own size.
    let title = root.join("Shots/MD/Streets of Rage 3 (USA).title.jpg");
    assert_eq!(size_of(&title), (320, 224));
    assert!(root.join("Shots/MD/Streets of Rage 3 (USA).play.jpg").is_file());

    // No temporary files are left.
    let parts = walk(&root).into_iter().filter(|p| p.extension().is_some_and(|e| e == "part"));
    assert_eq!(parts.count(), 0);
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

#[test]
fn a_second_run_uses_the_index_and_fetches_only_what_is_missing() {
    let (d, net, games) = setup(None);
    let root = d.path().join("Shelf");
    run(&root, &net, &games);
    assert_eq!(net.metadata_reads.load(Ordering::Relaxed), 1);
    let index = std::fs::read_to_string(root.join("Config/launchbox.txt")).unwrap();
    assert!(index.contains("SFC\tNot In LaunchBox (USA)\t?"), "{index}");

    std::fs::remove_file(root.join("Boxes/MD/Streets of Rage 3 (USA).back.jpg")).unwrap();
    net.asked.lock().unwrap().clear();
    let again = run(&root, &net, &games);
    assert_eq!(net.metadata_reads.load(Ordering::Relaxed), 1, "the metadata was read again");
    assert_eq!(*net.asked.lock().unwrap(), ["sor3-back.png"]);
    assert_eq!(again.pictures, 1);

    // An index from before the screenshots sends it back to LaunchBox.
    let old: String = index
        .lines()
        .map(|l| l.split('\t').take(5).collect::<Vec<_>>().join("\t") + "\n")
        .collect();
    std::fs::write(root.join("Config/launchbox.txt"), old).unwrap();
    run(&root, &net, &games);
    assert_eq!(net.metadata_reads.load(Ordering::Relaxed), 2);

    // A game the index does not know sends it back to LaunchBox.
    let mut more = games.clone();
    more.push((Platform::Md, "Streets of Rage 3 (Europe)".to_string()));
    run(&root, &net, &more);
    assert_eq!(net.metadata_reads.load(Ordering::Relaxed), 3);
}

#[test]
fn a_png_already_on_the_card_counts_as_there() {
    let (d, net, games) = setup(None);
    let root = d.path().join("Shelf");
    let png = root.join("Boxes/MD/Streets of Rage 3 (USA).front.png");
    std::fs::create_dir_all(png.parent().unwrap()).unwrap();
    std::fs::write(&png, png_of(10, 10, 0)).unwrap();
    assert!(Kind::Front.on_card(&root, &games[0]));
    run(&root, &net, &games);
    assert!(!net.asked.lock().unwrap().contains(&"sor3-eu.png".to_string()));
}

#[test]
fn a_busy_server_stops_the_run_with_a_reason() {
    let (d, net, games) = setup(Some("sor3-back.png"));
    let root = d.path().join("Shelf");
    let summary = run(&root, &net, &games);
    assert!(summary.stopped.is_some_and(|s| s.contains("busy")));
}

#[test]
fn the_estimate_counts_the_missing_pictures() {
    let (d, _net, games) = setup(None);
    let root = d.path().join("Shelf");
    let (n, bytes) = shelf_fetch::estimate(&root, &games);
    assert_eq!(n, 15);
    assert!(bytes > 0 && bytes < 2 << 20);
    assert!(shelf_fetch::free_space(d.path()).is_some_and(|f| f > 0));
}

const DISC_XML: &str = r#"<?xml version="1.0" standalone="yes"?>
<LaunchBox>
  <Game>
    <Name>Crash Bandicoot</Name>
    <DatabaseID>30</DatabaseID>
    <Platform>Sony Playstation</Platform>
  </Game>
  <GameImage>
    <DatabaseID>30</DatabaseID>
    <FileName>crash-cart.png</FileName>
    <Type>Cart - Front</Type>
  </GameImage>
  <GameImage>
    <DatabaseID>30</DatabaseID>
    <FileName>crash-disc-fan.png</FileName>
    <Type>Fanart - Disc</Type>
  </GameImage>
  <GameImage>
    <DatabaseID>30</DatabaseID>
    <FileName>crash-disc.png</FileName>
    <Type>Disc</Type>
    <Region>Japan</Region>
  </GameImage>
</LaunchBox>
"#;

fn disc_setup() -> (tempfile::TempDir, Arc<FakeNet>, Vec<shelf_fetch::Game>) {
    let d = tempfile::tempdir().unwrap();
    let zip = d.path().join("Metadata.zip");
    std::fs::write(
        &zip,
        zip_of(&[("Platforms.xml", b"<LaunchBox/>"), ("Metadata.xml", DISC_XML.as_bytes())]),
    )
    .unwrap();
    let net = Arc::new(FakeNet {
        zip,
        metadata_reads: AtomicUsize::new(0),
        asked: Mutex::new(Vec::new()),
        busy: None,
    });
    (d, net, vec![(Platform::Ps, "Crash Bandicoot (USA)".to_string())])
}

#[test]
fn a_cd_game_gets_its_disc_picture_and_never_a_cart_photo() {
    let (d, net, games) = disc_setup();
    let root = d.path().join("Shelf");
    run(&root, &net, &games);
    let asked = net.asked.lock().unwrap().clone();
    assert!(asked.contains(&"crash-disc.png".to_string()), "{asked:?}");
    assert!(!asked.contains(&"crash-cart.png".to_string()), "{asked:?}");
    assert!(!asked.contains(&"crash-disc-fan.png".to_string()), "{asked:?}");
    assert!(root.join("Carts/PS/Crash Bandicoot (USA).png").is_file());
}

#[test]
fn an_index_from_before_discs_is_read_again_for_the_disc_systems() {
    let (d, net, games) = disc_setup();
    let root = d.path().join("Shelf");
    run(&root, &net, &games);
    assert_eq!(net.metadata_reads.load(Ordering::Relaxed), 1);
    // A second run goes by the index alone.
    run(&root, &net, &games);
    assert_eq!(net.metadata_reads.load(Ordering::Relaxed), 1);
    // The same index without the line that marks discs is out of date.
    let index = root.join("Config/launchbox.txt");
    let old: String = std::fs::read_to_string(&index)
        .unwrap()
        .lines()
        .filter(|l| *l != "# discs: pictured")
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(&index, old).unwrap();
    run(&root, &net, &games);
    assert_eq!(net.metadata_reads.load(Ordering::Relaxed), 2);
}
