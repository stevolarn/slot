const INSERT: &[u8] = include_bytes!("../../assets/insert.pcm");
const ASSET_HZ: f32 = 48_000.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sfx {
    Insert,
    /// Shrink-wrap torn off a new game: made here, not recorded.
    Unwrap,
}

impl Sfx {
    pub fn lead(self) -> f32 {
        match self {
            Sfx::Insert => 0.097,
            Sfx::Unwrap => 0.0,
        }
    }

    pub fn tail(self) -> f32 {
        let pcm = match self {
            Sfx::Insert => INSERT,
            Sfx::Unwrap => return UNWRAP_SECS,
        };
        pcm.len() as f32 / 2.0 / ASSET_HZ - self.lead()
    }

    pub fn render(self, sample_rate: u32) -> Vec<i16> {
        let mono = match self {
            Sfx::Insert => resampled(INSERT, sample_rate),
            Sfx::Unwrap => unwrap(sample_rate),
        };
        let mut out = Vec::with_capacity(mono.len() * 2);
        for v in mono {
            let s = v.clamp(-32768.0, 32767.0) as i16;
            out.push(s);
            out.push(s);
        }
        out
    }
}

fn resampled(pcm: &[u8], sample_rate: u32) -> Vec<f32> {
    let src: Vec<f32> = pcm
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32)
        .collect();
    if sample_rate == ASSET_HZ as u32 || src.is_empty() {
        return src;
    }
    let ratio = sample_rate as f32 / ASSET_HZ;
    let n = (src.len() as f32 * ratio) as usize;
    (0..n)
        .map(|i| {
            let x = i as f32 / ratio;
            let a = x as usize;
            let f = x - a as f32;
            let b = (a + 1).min(src.len() - 1);
            src[a] * (1.0 - f) + src[b] * f
        })
        .collect()
}

/// How long the unwrapping sound is, in seconds.
const UNWRAP_SECS: f32 = 1.0;

/// A tiny repeatable noise source.
struct Noise(u32);

impl Noise {
    /// The next value, from -1 to 1.
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() + 1.0) / 2.0
    }
}

/// A band-pass filter whose centre can move as it runs.
struct BandPass {
    low: f32,
    band: f32,
}

impl BandPass {
    fn run(&mut self, input: f32, centre: f32, q: f32, rate: f32) -> f32 {
        let f = 2.0 * (std::f32::consts::PI * centre / rate).sin();
        let high = input - self.low - self.band / q;
        self.band += f * high;
        self.low += f * self.band;
        self.band
    }
}

/// Clicks a second of crinkling plastic gives, `t` seconds after A: a few
/// as it is grabbed, a flurry as the seam tears, a good many as the wrap
/// is pulled off and crumpled, and the last few after.
fn crackle_rate(t: f32) -> f32 {
    match t {
        t if t < 0.12 => 40.0,
        t if t < 0.38 => 170.0,
        t if t < 0.85 => 90.0,
        _ => 18.0,
    }
}

/// The sound of shrink-wrap coming off a box, made fresh: crackling
/// plastic, a rip along the seam, and a crumple as it is balled up.
fn unwrap(sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let n = (UNWRAP_SECS * rate) as usize;
    let mut noise = Noise(0x2545_f491);
    let mut out = vec![0.0f32; n];
    // Crackle: short bright clicks, scattered.
    let mut t = 0.0f32;
    while t < UNWRAP_SECS {
        t += -noise.unit().max(1e-4).ln() / crackle_rate(t);
        let start = (t * rate) as usize;
        let len = ((0.001 + 0.004 * noise.unit()) * rate) as usize;
        let amp = 0.15 + 0.55 * noise.unit();
        let mut last = 0.0;
        for k in 0..len {
            let Some(s) = out.get_mut(start + k) else { break };
            let v = noise.next();
            // The difference of the noise: crisp, with no thump to it.
            *s += (v - last) * amp * (-(k as f32) / (len as f32 * 0.3)).exp();
            last = v;
        }
    }
    // The rip: noise through a band rising as the seam opens.
    let mut rip = BandPass { low: 0.0, band: 0.0 };
    let mut crumple = BandPass { low: 0.0, band: 0.0 };
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / rate;
        let r = (t - 0.12) / 0.3;
        if (0.0..1.0).contains(&r) {
            let env = (r / 0.08).min(1.0) * (1.0 - r).powf(1.5);
            let centre = 1500.0 + 4000.0 * r;
            *s += rip.run(noise.next(), centre, 2.5, rate) * env * 0.9;
        }
        let c = (t - 0.38) / 0.55;
        if (0.0..1.0).contains(&c) {
            let wobble = 0.5 + 0.5 * (t * 37.0).sin() * (t * 11.0).sin();
            let env = (c / 0.1).min(1.0) * (1.0 - c) * wobble;
            *s += crumple.run(noise.next(), 2400.0, 1.2, rate) * env * 0.35;
        }
    }
    // A short fade at each end, and as loud as the cart going in.
    let fade = (0.004 * rate) as usize;
    for i in 0..fade.min(n) {
        let g = i as f32 / fade as f32;
        out[i] *= g;
        out[n - 1 - i] *= g;
    }
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    let gain = 0.55 * 32767.0 / peak;
    out.iter_mut().for_each(|v| *v *= gain);
    out
}

#[cfg(test)]
mod unwrap_tests {
    use super::*;

    #[test]
    fn the_unwrap_is_made_at_any_rate_and_is_not_silent() {
        for rate in [48_000u32, 44_100] {
            let pcm = Sfx::Unwrap.render(rate);
            assert_eq!(pcm.len(), 2 * (UNWRAP_SECS * rate as f32) as usize);
            let loud = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap();
            assert!(loud > 10_000, "too quiet at {rate}: {loud}");
            assert_eq!(pcm[0], 0, "clicks in at {rate}");
        }
    }

    #[test]
    fn the_unwrap_sounds_the_same_every_time() {
        assert_eq!(Sfx::Unwrap.render(48_000), Sfx::Unwrap.render(48_000));
    }
}
