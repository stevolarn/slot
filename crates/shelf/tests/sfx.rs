mod common;

use common::tmp_root_with_carts;
use shelf::app::{INSERT_S, SEATED_AT};
use shelf::audio::{ring_capacity, Ring, Sfx, OUTPUT_HZ};
use shelf::session::Session;

#[test]
fn the_contacts_in_the_clip_are_where_the_lead_says() {
    let s = Sfx::Insert;
    let v = s.render(48_000);
    let at = v
        .chunks_exact(2)
        .enumerate()
        .max_by_key(|(_, c)| c[0].unsigned_abs())
        .map(|(i, _)| i as f32 / 48_000.0)
        .unwrap();
    assert!(
        (at - s.lead()).abs() < 0.008,
        "{s:?}: the contacts are {:.0} ms into the clip and lead says {:.0} ms",
        at * 1000.0,
        s.lead() * 1000.0
    );
}

#[test]
fn the_insert_starts_inside_the_travel() {
    assert!(
        Sfx::Insert.lead() < SEATED_AT,
        "the clip is longer than the travel it belongs to"
    );
}

#[test]
fn the_clip_is_the_recording_and_nothing_else() {
    let s = Sfx::Insert;
    let a = s.render(48_000);
    let b = s.render(48_000);
    assert_eq!(a, b, "{s:?} is not the same twice");
    assert!(
        a.chunks_exact(2).all(|c| c[0] == c[1]),
        "{s:?} is not the mono recording"
    );
}

#[test]
fn the_clip_does_not_start_or_end_on_a_step() {
    let v = Sfx::Insert.render(48_000);
    assert!(v[..8].iter().all(|x| x.abs() < 400), "it starts on a step");
    assert!(
        v[v.len() - 200..].iter().all(|x| x.abs() < 400),
        "it ends on a step"
    );
}

#[test]
fn a_clip_is_mixed_into_what_is_already_queued_rather_than_played_after_it() {
    let r = Ring::new(ring_capacity(48_000));
    let c = Sfx::Insert.render(48_000);
    r.mix(&vec![1_000i16; c.len() * 2]);
    let queued = r.queued_frames();
    r.mix(&c);
    assert_eq!(
        r.queued_frames(),
        queued,
        "the clip was appended, so it plays after what was queued"
    );
    let mut out = vec![0i16; c.len()];
    r.fill(&mut out);
    assert_eq!(out[0], 1_000i16.saturating_add(c[0]));
}

#[test]
fn the_whole_of_a_cart_sound_reaches_the_device() {
    for rate in [48_000, OUTPUT_HZ] {
        for s in [Sfx::Insert] {
            let clip = s.render(rate);
            assert!(
                clip.len() > ring_capacity(rate) * 2,
                "{s:?} now fits the ring at {rate} Hz, so this no longer exercises anything"
            );
            let r = Ring::new(ring_capacity(rate));
            r.reopen(rate);
            r.mix(&clip);
            let mut got: Vec<i16> = Vec::new();
            while got.len() < clip.len() {
                let mut out = vec![0i16; 512 * 2];
                r.fill(&mut out);
                got.extend_from_slice(&out);
            }
            let first = got
                .iter()
                .zip(&clip)
                .position(|(a, b)| a != b)
                .unwrap_or(clip.len());
            assert_eq!(
                first,
                clip.len(),
                "{s:?} at {rate} Hz came apart {:.0} ms in, {:.0} ms short of the {:.0} ms it runs",
                1000.0 * first as f32 / 2.0 / rate as f32,
                1000.0 * (clip.len() - first) as f32 / 2.0 / rate as f32,
                1000.0 * clip.len() as f32 / 2.0 / rate as f32,
            );
        }
    }
}

#[test]
fn a_clip_plays_on_its_own() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut s = Session::boot(d.path().to_path_buf());
    s.play_sfx(Sfx::Insert);
    assert!(s.audio_queued() > 0, "the clip went nowhere");
}

#[test]
fn putting_a_cart_in_makes_one_sound() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = common::boot(d.path());
    common::insert(&mut a);
    let mut heard: Vec<Sfx> = a.take_sfx().into_iter().collect();
    for _ in 0..90 {
        a.update(1.0 / 60.0);
        if let Some(s) = a.take_sfx() {
            heard.push(s);
        }
    }
    assert_eq!(
        heard,
        vec![Sfx::Insert],
        "the insert sounded like {heard:?}"
    );
}

#[test]
fn the_hand_off_waits_for_the_cart_to_finish_landing() {
    let hold = INSERT_S - SEATED_AT;
    let tail = Sfx::Insert.tail();
    assert!(
        hold > tail,
        "the hand off comes {:.0} ms after the cart lands and the sound runs {:.0} ms",
        hold * 1000.0,
        tail * 1000.0
    );
    let beat = hold - tail;
    assert!(
        (0.05..0.30).contains(&beat),
        "{:.0} ms of air between the sound and the hand off",
        beat * 1000.0
    );
}

fn sfx_peak(setup: impl Fn(&mut Session)) -> u16 {
    let d = tmp_root_with_carts(&["Emerald"]);
    shelf_store::write_slot_state(
        d.path(),
        &shelf_store::SlotState {
            clock_set: true,
            ..shelf_store::SlotState::default()
        },
    )
    .unwrap();
    let mut s = Session::boot(d.path().to_path_buf());
    setup(&mut s);
    s.play_sfx(Sfx::Insert);
    let ring = s.audio_ring();
    let mut out = vec![0i16; ring.queued_frames() * 2];
    ring.fill(&mut out);
    out.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0)
}

#[test]
fn a_cart_sound_is_played_at_the_volume_that_is_set() {
    let loud = sfx_peak(|_| {});
    let quiet = sfx_peak(|s| {
        for _ in 0..8 {
            s.app_mut().apply(shelf_input::Action::VolumeDown);
        }
    });
    assert!(loud > 0, "the clip was silent at the volume it booted with");
    assert!(
        quiet * 2 < loud,
        "turning the volume down did not quieten the cart: {quiet} against {loud}"
    );
}

#[test]
fn a_cart_sound_is_silent_when_muted() {
    let muted = sfx_peak(|s| s.app_mut().apply(shelf_input::Action::MuteToggle));
    assert_eq!(muted, 0, "the cart was heard through a mute");
}
