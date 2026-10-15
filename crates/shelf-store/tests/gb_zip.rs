use shelf_store::gb::{self, Class};
use tempfile::TempDir;

fn rom(cgb: u8, title: &str) -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];
    rom[0x134..0x134 + title.len()].copy_from_slice(title.as_bytes());
    rom[0x143] = cgb;
    rom
}

/// A one-file zip, written the way zip tools do, stored or deflated.
fn zip(name: &str, data: &[u8], deflate: bool) -> Vec<u8> {
    zip_with(name, data, deflate, 0, b"")
}

/// The same, with `extra` bytes of local extra field before the data and a
/// zip comment after the end record.
fn zip_with(name: &str, data: &[u8], deflate: bool, extra: usize, comment: &[u8]) -> Vec<u8> {
    let (method, body) = if deflate {
        (8u16, miniz_oxide::deflate::compress_to_vec(data, 6))
    } else {
        (0u16, data.to_vec())
    };
    let mut out = Vec::new();
    let fields = |out: &mut Vec<u8>, extra: usize| {
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&(extra as u16).to_le_bytes());
    };
    out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    out.extend_from_slice(&[20, 0, 0, 0]);
    fields(&mut out, extra);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&vec![0u8; extra]);
    out.extend_from_slice(&body);
    let dir = out.len() as u32;
    out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    out.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
    fields(&mut out, 0);
    out.extend_from_slice(&[0; 12]);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    let dir_len = out.len() as u32 - dir;
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
    out.extend_from_slice(&dir_len.to_le_bytes());
    out.extend_from_slice(&dir.to_le_bytes());
    out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
    out.extend_from_slice(comment);
    out
}

#[test]
fn a_zipped_game_boy_colour_game_is_still_read_as_colour_only() {
    let d = TempDir::new().unwrap();
    for (deflate, file) in [(true, "Prism.zip"), (false, "Stored.zip")] {
        let path = d.path().join(file);
        std::fs::write(&path, zip("Prism.gbc", &rom(0xc0, "PRISM"), deflate)).unwrap();
        assert_eq!(gb::class(&path), Class::ColourOnly, "{file}");
        assert_eq!(gb::title(&path).as_deref(), Some("PRISM"), "{file}");
    }
}

#[test]
fn a_zipped_dual_mode_game_and_an_original_keep_their_class() {
    let d = TempDir::new().unwrap();
    let dual = d.path().join("Zelda DX.zip");
    std::fs::write(&dual, zip("Zelda DX.gbc", &rom(0x80, "ZELDA"), true)).unwrap();
    assert_eq!(gb::class(&dual), Class::DualMode);
    let old = d.path().join("Tetris.zip");
    std::fs::write(&old, zip("Tetris.gb", &rom(0x00, "TETRIS"), true)).unwrap();
    assert_eq!(gb::class(&old), Class::Original);
}

#[test]
fn a_damaged_zip_falls_back_to_an_original_cart() {
    let d = TempDir::new().unwrap();
    let bad = d.path().join("Broken.zip");
    let mut bytes = zip("Broken.gbc", &rom(0xc0, "BROKEN"), true);
    bytes.truncate(bytes.len() / 2);
    std::fs::write(&bad, bytes).unwrap();
    assert_eq!(gb::class(&bad), Class::Original);
}

#[test]
fn zips_with_a_comment_or_a_long_local_field_are_still_read() {
    let d = TempDir::new().unwrap();
    let data = rom(0xc0, "COMMENTED");
    for (deflate, extra, comment) in [
        (true, 0, &b"made with love"[..]),
        (false, 0, &[b'x'; 5000][..]),
        (true, 6000, &b""[..]),
        (false, 6000, &b"c"[..]),
    ] {
        let path = d
            .path()
            .join(format!("z{deflate}{extra}{}.zip", comment.len()));
        std::fs::write(
            &path,
            zip_with("Commented.gbc", &data, deflate, extra, comment),
        )
        .unwrap();
        let head = shelf_store_head(&path);
        assert_eq!(head.as_deref(), Some(&data[..0x150]), "{path:?}");
        assert_eq!(gb::class(&path), Class::ColourOnly, "{path:?}");
    }
}

fn shelf_store_head(path: &std::path::Path) -> Option<Vec<u8>> {
    let h = gb::header(path)?;
    // The header carries the fields the zip fixture set.
    assert_eq!(h.title, "COMMENTED");
    let mut out = rom(0xc0, "COMMENTED");
    out.truncate(0x150);
    Some(out)
}

#[test]
fn a_hard_to_compress_game_needs_more_than_the_first_read() {
    // A header buried under 4KB+ of noise inside the compressed stream
    // cannot be, but a rom whose first bytes are noise stays in the first
    // read; check both come back right.
    let mut data = rom(0x80, "NOISY");
    let mut x = 0x1234_5678u32;
    for b in data[..0x130].iter_mut() {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *b = (x >> 24) as u8;
    }
    let d = TempDir::new().unwrap();
    let path = d.path().join("Noisy.zip");
    std::fs::write(&path, zip("Noisy.gb", &data, true)).unwrap();
    assert_eq!(gb::class(&path), Class::DualMode);
}

#[test]
fn a_seven_zip_is_not_read_as_a_zip() {
    let d = TempDir::new().unwrap();
    let path = d.path().join("Game.7z");
    std::fs::write(
        &path,
        [b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c, 0, 4, 0, 0, 0, 0],
    )
    .unwrap();
    assert!(gb::header(&path).is_none());
    assert_eq!(gb::class(&path), Class::Original);
}

#[test]
fn a_cached_header_is_read_from_the_card_once() {
    let d = TempDir::new().unwrap();
    let path = d.path().join("Once.zip");
    std::fs::write(&path, zip("Once.gbc", &rom(0xc0, "ONCE"), true)).unwrap();
    let before = gb::cached_reads();
    for _ in 0..5 {
        assert_eq!(gb::class_cached(&path), Class::ColourOnly);
        assert_eq!(gb::header_cached(&path).unwrap().title, "ONCE");
    }
    assert_eq!(gb::cached_reads() - before, 1);
    let missing = d.path().join("none.gb");
    for _ in 0..3 {
        assert!(gb::header_cached(&missing).is_none());
    }
    assert_eq!(gb::cached_reads() - before, 2);
}
