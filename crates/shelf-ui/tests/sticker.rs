use shelf_ui::{sticker_lines, StickerFields};

fn fields() -> StickerFields<'static> {
    StickerFields {
        battery: Some(87),
        serial: "0473885",
        dirty_digit: '0',
    }
}

#[test]
fn the_headline_rows_read_as_a_device_plate() {
    let all = sticker_lines(&fields()).join("\n");
    assert!(all.contains("SHELF-TG5040"), "{all}");
    assert!(all.contains("5V"), "{all}");
    assert!(all.contains("1.5A"), "{all}");
    assert!(all.contains("87"), "the gauge reading is missing: {all}");
    assert!(all.contains("0473885"), "the serial went missing: {all}");
}

#[test]
fn the_input_row_carries_the_real_dc_symbol() {
    let all = sticker_lines(&fields()).join("\n");
    assert!(all.contains(shelf_ui::DC), "the rating row lost its symbol");
    assert_eq!(shelf_ui::DC, '\u{2393}');
}

#[test]
fn a_missing_gauge_is_not_drawn_as_empty() {
    let mut f = fields();
    f.battery = None;
    let all = sticker_lines(&f).join("\n");
    assert!(
        !all.contains("0%"),
        "no gauge was drawn as a flat battery: {all}"
    );
    assert!(
        all.contains("BATTERY"),
        "the row should still be there: {all}"
    );
}
#[test]
fn the_compliance_block_is_the_credits() {
    let all = sticker_lines(&fields()).join("\n").to_uppercase();
    for owed in [
        "NEXTUI",
        "TRIMUI BRICK",
        "SLOT BY BRANDON T.",
        "GPL-3.0",
        "OPEN SANS",
        "NERD",
        "CLAUDE",
    ] {
        assert!(all.contains(owed), "the credits do not mention {owed}");
    }
}

#[test]
fn the_credits_leave_out_what_shelf_no_longer_ships() {
    let all = sticker_lines(&fields()).join("\n").to_uppercase();
    for gone in ["MGBA", "GPSP", "BASEOS", "LCD3X"] {
        assert!(!all.contains(gone), "the credits still mention {gone}");
    }
}

#[test]
fn the_original_copyright_notice_stays_on_the_sticker() {
    let all = sticker_lines(&fields()).join("\n");
    assert!(
        all.contains("\u{a9} 2026 BRANDON T. KOWALSKI"),
        "the original copyright notice is gone: {all}"
    );
    assert!(all.contains("GPL-3.0"), "{all}");
}

#[test]
fn the_wordmark_is_drawn_in_antialiased_ink() {
    use shelf_ui::sticker_face;
    const GROUND: [u8; 3] = [0x23, 0x1f, 0x20];
    const INK: [u8; 3] = [0xff, 0xff, 0xff];
    let face = sticker_face(&fields());
    let mut ink = 0;
    let mut soft = 0;
    for y in 128..176 {
        for x in 380..640 {
            let i = ((y * face.w + x) * 4) as usize;
            let px = [face.rgba[i], face.rgba[i + 1], face.rgba[i + 2]];
            if px == INK {
                ink += 1;
            } else if px != GROUND {
                soft += 1;
            }
        }
    }
    assert!(ink > 100, "the wordmark was not drawn: {ink} px of ink");
    assert!(
        soft > 20,
        "the wordmark's edges are hard steps: {soft} soft px"
    );
}

#[test]
fn the_serial_row_matches_the_encoded_hash() {
    let all = sticker_lines(&fields()).join("\n");
    assert!(all.contains("0473885"), "{all}");
}

#[test]
fn every_line_is_already_upper_case() {
    for line in sticker_lines(&fields()) {
        assert_eq!(line, line.to_uppercase(), "{line}");
    }
}
