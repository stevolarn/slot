use std::time::Duration;

use slot::audio::{ring_capacity, AudioSink, Ring, StubSink, OUTPUT_HZ};

fn ramp(frames: usize, from: i16) -> Vec<i16> {
    (0..frames * 2)
        .map(|i| from.wrapping_add(i as i16))
        .collect()
}

#[test]
fn queued_frames_tracks_what_the_device_has_yet_to_play() {
    let mut s = StubSink::new();
    s.open(32768).unwrap();
    let r = s.ring();
    assert_eq!(
        r.queued_frames(),
        0,
        "something was queued before any sound"
    );
    r.mix(&ramp(1000, 1));
    assert_eq!(r.queued_frames(), 1000);
    s.device_read(600);
    assert_eq!(r.queued_frames(), 400);
}

#[test]
fn opening_leaves_the_ring_silent() {
    let mut s = StubSink::new();
    s.open(48_000).unwrap();
    assert_eq!(s.ring().queued_frames(), 0);
    assert!(s.device_read(64).iter().all(|v| *v == 0));
}

#[test]
fn opening_remembers_the_rate_and_sizes_the_ring_to_it() {
    let mut s = StubSink::new();
    s.open(48_000).unwrap();
    let r = s.ring();
    assert_eq!(r.sample_rate(), 48_000);
    assert_eq!(r.capacity_frames(), ring_capacity(48_000));
}

#[test]
fn the_ring_holds_enough_for_a_whole_cart_sound_in_one_go() {
    let ms = ring_capacity(48_000) as f64 / 48_000.0 * 1000.0;
    assert!(ms > 100.0, "only {ms:.0} ms of room");
}

#[test]
fn samples_come_back_in_order_across_the_wrap() {
    let r = Ring::new(8);
    let first = ramp(6, 1);
    r.mix(&first);

    let mut out = vec![0i16; 8];
    r.fill(&mut out);
    assert_eq!(out, first[..8]);

    let mut rest = vec![0i16; 8];
    r.fill(&mut rest);
    assert_eq!(
        rest[..4],
        first[8..],
        "the end of the sound came back wrong"
    );
    assert!(
        rest[4..].iter().all(|s| *s == 0),
        "a tail of silence was invented"
    );

    let second = ramp(5, 100);
    r.mix(&second);
    let mut again = vec![0i16; 10];
    r.fill(&mut again);
    assert_eq!(
        again, second,
        "a sound laid across the wrap came back out of order"
    );
}

#[test]
fn a_sound_laid_while_another_is_playing_adds_to_what_is_left_of_it() {
    let r = Ring::new(8);
    r.mix(&ramp(6, 1));
    let mut out = vec![0i16; 8];
    r.fill(&mut out);
    let left = ramp(6, 1)[8..].to_vec();

    r.mix(&[1000i16; 6]);
    let mut rest = vec![0i16; 6];
    r.fill(&mut rest);
    let want: Vec<i16> = left
        .iter()
        .map(|s| s.saturating_add(1000))
        .chain([1000, 1000])
        .collect();
    assert_eq!(rest, want);
}

#[test]
fn a_sound_longer_than_the_ring_is_laid_as_the_device_makes_room() {
    let r = Ring::new(4);
    let sound = ramp(10, 1);
    r.mix(&sound);
    assert_eq!(r.queued_frames(), 4, "more than the ring holds was queued");

    let mut got = Vec::new();
    while got.len() < sound.len() {
        let mut out = vec![0i16; 4];
        r.fill(&mut out);
        got.extend_from_slice(&out);
    }
    assert_eq!(got[..sound.len()], sound[..]);
}

#[test]
fn a_dry_ring_plays_silence_rather_than_repeating() {
    let r = Ring::new(64);
    r.mix(&ramp(2, 9));
    let mut out = vec![-1i16; 8];
    r.fill(&mut out);
    assert_eq!(&out[..4], &ramp(2, 9)[..]);
    assert_eq!(&out[4..], &[0, 0, 0, 0]);
}

#[test]
fn a_cart_sound_plays_at_once_from_a_silent_ring() {
    let r = Ring::new(ring_capacity(48_000));
    let mut out = vec![0i16; 512 * 2];
    r.fill(&mut out);
    r.mix(&ramp(100, 1));
    r.fill(&mut out);
    assert!(
        out.iter().any(|s| *s != 0),
        "the cart sound was swallowed before it played"
    );
}

#[test]
fn the_ring_at_an_odd_capacity_still_ends_on_a_frame() {
    assert_eq!(
        ring_capacity(OUTPUT_HZ) % 2,
        1,
        "this rate no longer has an odd capacity, so it no longer exercises the bug"
    );
    let r = Ring::new(ring_capacity(OUTPUT_HZ));
    r.reopen(OUTPUT_HZ);

    let mut idle = vec![1i16; 1024];
    r.fill(&mut idle);
    assert!(idle.iter().all(|s| *s == 0), "an idle ring was not silent");
    let block: Vec<i16> = (0..64i16).flat_map(|k| [1000 + k, -1000 - k]).collect();
    r.mix(&block);
    let mut out = vec![0i16; block.len()];
    r.fill(&mut out);
    assert_eq!(
        out, block,
        "the ring ended mid frame, so left and right came back swapped"
    );
}

#[test]
fn silence_releases_the_device_only_after_three_quiet_seconds() {
    let mut s = slot::audio::Silence::new(Duration::from_secs(3));
    let period = Duration::from_millis(500);
    let quiet = [0i16; 64];
    for _ in 0..5 {
        assert!(s.hear(&quiet, period));
    }
    assert!(!s.hear(&quiet, period), "still holding after 3s of silence");
    assert!(
        s.hear(&[0, 0, 3, 0], period),
        "sound did not reclaim the device"
    );
    assert!(s.hear(&quiet, period), "one sound did not restart the wait");
}
