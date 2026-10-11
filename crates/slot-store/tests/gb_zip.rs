use slot_store::gb::{self, Class};
use tempfile::TempDir;

fn rom(cgb: u8, title: &str) -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];
    rom[0x134..0x134 + title.len()].copy_from_slice(title.as_bytes());
    rom[0x143] = cgb;
    rom
}

/// A one-file zip, written the way zip tools do, stored or deflated.
fn zip(name: &str, data: &[u8], deflate: bool) -> Vec<u8> {
    let (method, body) = if deflate {
        (8u16, miniz_oxide::deflate::compress_to_vec(data, 6))
    } else {
        (0u16, data.to_vec())
    };
    let mut out = Vec::new();
    let fields = |out: &mut Vec<u8>| {
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
    };
    out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    out.extend_from_slice(&[20, 0, 0, 0]);
    fields(&mut out);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&body);
    let dir = out.len() as u32;
    out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    out.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
    fields(&mut out);
    out.extend_from_slice(&[0; 12]);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    let dir_len = out.len() as u32 - dir;
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
    out.extend_from_slice(&dir_len.to_le_bytes());
    out.extend_from_slice(&dir.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
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
