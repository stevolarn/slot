mod common;

use common::tmp_root;
use slot_store::{atomic_write, read_slot_state, write_slot_state, Platform, SlotState};
use tempfile::tempdir;

#[test]
fn atomic_write_leaves_no_partial_file_and_no_temp_behind() {
    let d = tempdir().unwrap();
    let p = d.path().join("x.bin");
    atomic_write(&p, b"first").unwrap();
    atomic_write(&p, &vec![7u8; 4_000_000]).unwrap();
    assert_eq!(std::fs::read(&p).unwrap().len(), 4_000_000);
    let strays: Vec<_> = std::fs::read_dir(d.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() != "x.bin")
        .collect();
    assert!(strays.is_empty(), "temp files left behind: {strays:?}");
}

#[test]
fn a_name_the_card_accepts_is_a_name_that_can_be_written() {
    let d = tempdir().unwrap();
    for len in [8usize, 100, 200, 239, 240, 245, 250, 251] {
        let p = d.path().join(format!("{}.sav", "a".repeat(len)));
        if std::fs::write(&p, b"control").is_err() {
            continue;
        }
        atomic_write(&p, b"save")
            .unwrap_or_else(|e| panic!("{len} character name: {e}, and a plain write took it"));
        assert_eq!(std::fs::read(&p).unwrap(), b"save");
    }
}

#[test]
fn a_long_multibyte_name_is_written_rather_than_panicking() {
    let d = tempdir().unwrap();
    for pad in 0..3 {
        let p = d
            .path()
            .join(format!("{}{}.sav", "x".repeat(pad), "ポ".repeat(82)));
        if std::fs::write(&p, b"control").is_err() {
            continue;
        }
        atomic_write(&p, b"save")
            .unwrap_or_else(|e| panic!("a Japanese cart name, pad {pad}: {e}"));
        assert_eq!(std::fs::read(&p).unwrap(), b"save");
    }
}

#[test]
fn corrupt_slot_state_reads_as_default_rather_than_panicking() {
    let d = tmp_root();
    std::fs::write(d.path().join("Config/slot.state"), b"\x00\xff not json").unwrap();
    assert_eq!(read_slot_state(d.path()), SlotState::default());
}

#[test]
fn slot_state_round_trips_including_a_stem_with_an_equals_sign() {
    let d = tmp_root();
    let s = SlotState {
        cart: Some("Cheats = On".into()),
        brightness: 3,
        volume: 71,
        muted: true,
        clock_set: true,
        utc_offset_min: 0,
        ..SlotState::default()
    };
    write_slot_state(d.path(), &s).unwrap();
    assert_eq!(read_slot_state(d.path()), s);
}

#[test]
fn a_slot_state_missing_a_key_reads_as_default_not_half_populated() {
    let d = tmp_root();
    std::fs::write(
        d.path().join("Config/slot.state"),
        "cart=Emerald\nbrightness=3\n",
    )
    .unwrap();
    assert_eq!(read_slot_state(d.path()), SlotState::default());
}

#[test]
fn an_out_of_range_level_reads_as_default() {
    let d = tmp_root();
    for body in [
        "cart=\nbrightness=10\nvolume=50\nmuted=0\nclock_set=1\nutc_offset_min=0\n",
        "cart=\nbrightness=3\nvolume=101\nmuted=0\nclock_set=1\nutc_offset_min=0\n",
        "cart=\nbrightness=3\nvolume=50\nmuted=2\nclock_set=1\nutc_offset_min=0\n",
    ] {
        std::fs::write(d.path().join("Config/slot.state"), body).unwrap();
        assert_eq!(
            read_slot_state(d.path()),
            SlotState::default(),
            "accepted {body:?}"
        );
    }
}

#[test]
fn a_first_boot_is_neither_dark_nor_silent() {
    let d = tmp_root();
    let s = read_slot_state(d.path());
    assert!(s.cart.is_none());
    assert!(s.brightness > 0, "boots with the backlight off");
    assert!(s.volume > 0, "boots muted");
}

#[test]
fn slot_state_round_trips_a_negative_utc_offset() {
    let d = tmp_root();
    let s = SlotState {
        cart: None,
        brightness: 5,
        volume: 60,
        muted: false,
        clock_set: true,
        utc_offset_min: -450,
        ..SlotState::default()
    };
    write_slot_state(d.path(), &s).unwrap();
    assert_eq!(read_slot_state(d.path()).utc_offset_min, -450);
}

#[test]
fn a_line_the_reader_does_not_know_is_skipped() {
    let d = tmp_root();
    std::fs::write(
        d.path().join("Config/slot.state"),
        "cart=Emerald\nbrightness=3\nfrom_a_later_build=7\nvolume=40\nmuted=1\n\
         no equals sign at all\nclock_set=1\nutc_offset_min=-300\n",
    )
    .unwrap();
    let s = read_slot_state(d.path());
    assert_eq!(
        s.cart.as_deref(),
        Some("Emerald"),
        "the file was thrown away"
    );
    assert_eq!(
        (
            s.brightness,
            s.volume,
            s.muted,
            s.clock_set,
            s.utc_offset_min
        ),
        (3, 40, true, true, -300)
    );
}

#[test]
fn a_first_boot_has_the_scanlines_off() {
    assert!(!SlotState::default().crt);
}

#[test]
fn the_crt_setting_round_trips_as_its_own_line() {
    let d = tmp_root();
    let s = SlotState {
        clock_set: true,
        crt: true,
        ..SlotState::default()
    };
    write_slot_state(d.path(), &s).unwrap();
    assert_eq!(read_slot_state(d.path()), s);
    let text = std::fs::read_to_string(d.path().join("Config/slot.state")).unwrap();
    assert!(text.lines().any(|l| l == "crt=1"), "no crt=1 in {text:?}");
}

#[test]
fn an_out_of_range_crt_falls_back_to_off_without_losing_the_rest() {
    let d = tmp_root();
    let known = "cart=Emerald\nbrightness=3\nvolume=40\nmuted=0\nclock_set=1\nutc_offset_min=0\n";
    for bad in ["crt=2\n", "crt=\n", "crt=on\n", "crt=-1\n"] {
        std::fs::write(d.path().join("Config/slot.state"), format!("{known}{bad}")).unwrap();
        let s = read_slot_state(d.path());
        assert!(!s.crt, "accepted {bad:?}");
        assert_eq!(
            (s.cart.as_deref(), s.brightness, s.volume, s.clock_set),
            (Some("Emerald"), 3, 40, true),
            "{bad:?} took the rest of the card with it"
        );
    }
}

#[test]
fn a_card_from_before_the_crt_setting_keeps_all_its_values() {
    let d = tmp_root();
    std::fs::write(
        d.path().join("Config/slot.state"),
        "cart=Emerald\nbrightness=3\nvolume=40\nmuted=1\nclock_set=1\nutc_offset_min=-300\n",
    )
    .unwrap();
    assert_eq!(
        read_slot_state(d.path()),
        SlotState {
            cart: Some("Emerald".into()),
            cart_platform: None,
            brightness: 3,
            volume: 40,
            muted: true,
            clock_set: true,
            utc_offset_min: -300,
            crt: false,
        }
    );
}

#[test]
fn every_platform_round_trips_as_its_own_line() {
    for platform in Platform::ALL {
        let d = tmp_root();
        let s = SlotState {
            cart: Some("Tetris".into()),
            cart_platform: Some(platform),
            clock_set: true,
            ..SlotState::default()
        };
        write_slot_state(d.path(), &s).unwrap();
        assert_eq!(
            read_slot_state(d.path()),
            s,
            "{platform:?} did not come back"
        );
        let text = std::fs::read_to_string(d.path().join("Config/slot.state")).unwrap();
        let want = format!("cart_platform={}", platform.dir_name().to_lowercase());
        assert!(text.lines().any(|l| l == want), "no {want} in {text:?}");
    }
}

#[test]
fn an_empty_slot_writes_an_empty_platform() {
    let d = tmp_root();
    let s = SlotState {
        clock_set: true,
        ..SlotState::default()
    };
    write_slot_state(d.path(), &s).unwrap();
    let text = std::fs::read_to_string(d.path().join("Config/slot.state")).unwrap();
    assert!(
        text.lines().any(|l| l == "cart_platform="),
        "no empty cart_platform line in {text:?}"
    );
    assert_eq!(read_slot_state(d.path()).cart_platform, None);
}

#[test]
fn an_unreadable_platform_reads_as_a_card_that_never_said() {
    let d = tmp_root();
    let known = "cart=Tetris\nbrightness=3\nvolume=40\nmuted=0\nclock_set=1\nutc_offset_min=0\n";
    for (line, want) in [
        ("cart_platform=\n", None),
        ("cart_platform=nes\n", None),
        ("cart_platform=gameboy\n", None),
        ("cart_platform=0\n", None),
        ("cart_platform=nds\n", None),
        ("cart_platform=GBA\n", Some(Platform::Gba)),
        ("cart_platform=Gbc\n", Some(Platform::Gbc)),
    ] {
        std::fs::write(d.path().join("Config/slot.state"), format!("{known}{line}")).unwrap();
        let s = read_slot_state(d.path());
        assert_eq!(s.cart_platform, want, "read {line:?} wrong");
        assert_eq!(
            (s.cart.as_deref(), s.brightness, s.volume, s.clock_set),
            (Some("Tetris"), 3, 40, true),
            "{line:?} took the rest of the card with it"
        );
    }
}

#[test]
fn an_offset_outside_the_range_of_real_zones_reads_as_default() {
    let d = tmp_root();
    for body in [
        "cart=\nbrightness=5\nvolume=60\nmuted=0\nclock_set=1\nutc_offset_min=900\n",
        "cart=\nbrightness=5\nvolume=60\nmuted=0\nclock_set=1\nutc_offset_min=-780\n",
    ] {
        std::fs::write(d.path().join("Config/slot.state"), body).unwrap();
        assert_eq!(read_slot_state(d.path()), SlotState::default(), "{body}");
    }
}
