use slot_store::Platform;

use super::console_sfx::Seat;

const INSERT: &[u8] = include_bytes!("../../assets/insert.pcm");
const EJECT: &[u8] = include_bytes!("../../assets/eject.pcm");
const ASSET_HZ: f32 = 48_000.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sfx {
    Insert,
    Eject,
    /// A NES, Mega Drive, SNES or Game Boy taking its cart, made here.
    Seat(Seat),
}

impl Sfx {
    /// The sound of the console the cart is for taking it. The GBA keeps
    /// the recorded clip.
    pub fn insert_for(platform: Option<Platform>) -> Sfx {
        match platform {
            Some(Platform::Nes) => Sfx::Seat(Seat::Nes),
            Some(Platform::Md) => Sfx::Seat(Seat::Md),
            Some(Platform::Snes) => Sfx::Seat(Seat::Snes),
            Some(Platform::Gb | Platform::Gbc) => Sfx::Seat(Seat::Gb),
            Some(Platform::Gba) | None => Sfx::Insert,
        }
    }

    pub fn lead(self) -> f32 {
        match self {
            Sfx::Seat(seat) => seat.lead(),
            Sfx::Insert => 0.097,
            Sfx::Eject => 0.021,
        }
    }

    fn samples(self) -> Vec<f32> {
        let pcm = match self {
            Sfx::Insert => INSERT,
            Sfx::Eject => EJECT,
            Sfx::Seat(seat) => return seat.render(),
        };
        pcm.chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32)
            .collect()
    }

    pub fn tail(self) -> f32 {
        self.samples().len() as f32 / ASSET_HZ - self.lead()
    }

    pub fn render(self, sample_rate: u32) -> Vec<i16> {
        let src = self.samples();
        let mut out = Vec::with_capacity(src.len() * 2);
        for v in resampled(src, sample_rate) {
            let s = v.clamp(-32768.0, 32767.0) as i16;
            out.push(s);
            out.push(s);
        }
        out
    }
}

fn resampled(src: Vec<f32>, sample_rate: u32) -> Vec<f32> {
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
