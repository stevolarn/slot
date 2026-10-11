//! The sound of each console taking its cart, made here from noise and
//! tones rather than recorded: a NES cart slid in and pushed down until
//! the tray locks, a Mega Drive cart pushed through its dust flaps, a
//! SNES cart dropped onto its pins, a Game Boy cart clicking home. Each
//! clip is loudest at the moment the cart seats, `lead` seconds in.

pub const HZ: f32 = 48_000.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Seat {
    Nes,
    Md,
    Snes,
    Gb,
}

impl Seat {
    pub fn lead(self) -> f32 {
        match self {
            Seat::Nes => 0.40,
            Seat::Md => 0.14,
            Seat::Snes => 0.16,
            Seat::Gb => 0.08,
        }
    }

    fn length(self) -> f32 {
        self.lead()
            + match self {
                Seat::Nes => 0.40,
                Seat::Md => 0.22,
                Seat::Snes => 0.36,
                Seat::Gb => 0.14,
            }
    }

    /// The clip at 48 kHz, mono, full scale ±32767.
    pub fn render(self) -> Vec<f32> {
        let mut c = Clip::new(self.length());
        let at = self.lead();
        match self {
            Seat::Nes => {
                // Slid in along the tray: a long soft scrape that ends on
                // the stop at the back...
                c.scrape(0.02, 0.20, 0.10, 0.55);
                c.knock(0.22, 900.0, 0.03, 0.30);
                c.thump(0.22, 140.0, 0.05, 0.25);
                // ...then pushed down against the spring, which creaks,
                // and locks with a heavy clunk.
                c.scrape(0.27, at - 0.27, 0.06, 0.25);
                c.thump(at, 95.0, 0.12, 0.95);
                c.knock(at, 1700.0, 0.02, 0.70);
                c.knock(at + 0.012, 2600.0, 0.01, 0.35);
            }
            Seat::Md => {
                // Pushed through the spring-loaded flaps...
                c.scrape(0.01, 0.10, 0.25, 0.45);
                c.knock(0.02, 2200.0, 0.012, 0.35);
                // ...and snapped onto the edge connector.
                c.knock(at, 1500.0, 0.025, 0.85);
                c.thump(at, 180.0, 0.07, 0.70);
                c.knock(at + 0.09, 2000.0, 0.01, 0.20);
            }
            Seat::Snes => {
                // Lowered into the slot...
                c.scrape(0.02, 0.12, 0.12, 0.35);
                // ...and set down firmly on the pins: a solid, deep thud.
                c.thump(at, 120.0, 0.11, 0.95);
                c.knock(at, 1100.0, 0.03, 0.55);
                c.thump(at + 0.004, 240.0, 0.04, 0.30);
            }
            Seat::Gb => {
                // A short slide and a bright plastic click.
                c.scrape(0.01, 0.06, 0.40, 0.35);
                c.knock(at, 2800.0, 0.012, 0.90);
                c.knock(at + 0.008, 4200.0, 0.006, 0.40);
                c.thump(at, 320.0, 0.03, 0.30);
            }
        }
        c.finish()
    }
}

struct Clip {
    s: Vec<f32>,
    seed: u32,
}

impl Clip {
    fn new(secs: f32) -> Self {
        Clip {
            s: vec![0.0; (secs * HZ) as usize],
            seed: 0x1234_5678,
        }
    }

    fn noise(&mut self) -> f32 {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }

    fn add(&mut self, i: usize, v: f32) {
        if let Some(x) = self.s.get_mut(i) {
            *x += v;
        }
    }

    /// Plastic sliding on plastic: noise, smoothed by `soft` (0 rough,
    /// 1 dull), rising and falling over `secs` from `from`.
    fn scrape(&mut self, from: f32, secs: f32, bright: f32, level: f32) {
        let (start, n) = ((from * HZ) as usize, (secs.max(0.0) * HZ) as usize);
        let mut low = 0.0;
        let k = 0.02 + 0.5 * bright;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let shape = (t * std::f32::consts::PI).sin();
            low += k * (self.noise() - low);
            let grain = 0.6 + 0.4 * self.noise().abs();
            self.add(start + i, low * shape * grain * level * 0.25);
        }
    }

    /// A hard edge striking: a burst of noise and a ringing tone at
    /// `freq`, both dying away over about `decay` seconds.
    fn knock(&mut self, at: f32, freq: f32, decay: f32, level: f32) {
        let start = (at * HZ) as usize;
        let n = (decay * 6.0 * HZ) as usize;
        for i in 0..n {
            let t = i as f32 / HZ;
            let env = (-t / decay).exp();
            let ring = (std::f32::consts::TAU * freq * t).sin();
            let hit = self.noise() * (-t / (decay * 0.25)).exp();
            self.add(start + i, (0.55 * ring + 0.45 * hit) * env * level);
        }
    }

    /// The body of the console answering: a low tone that drops a little
    /// in pitch as it dies over about `decay` seconds.
    fn thump(&mut self, at: f32, freq: f32, decay: f32, level: f32) {
        let start = (at * HZ) as usize;
        let n = (decay * 6.0 * HZ) as usize;
        let mut phase = 0.0;
        for i in 0..n {
            let t = i as f32 / HZ;
            let env = (-t / decay).exp() * (t / 0.002).min(1.0);
            phase += std::f32::consts::TAU * freq * (1.0 - 0.3 * (t / decay).min(1.0)) / HZ;
            self.add(start + i, phase.sin() * env * level);
        }
    }

    /// Scaled so its loudest sample is just under full scale, with the
    /// ends faded so it neither starts nor stops on a step.
    fn finish(mut self) -> Vec<f32> {
        let peak = self.s.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
        let n = self.s.len();
        let (fade_in, fade_out) = ((0.004 * HZ) as usize, (0.04 * HZ) as usize);
        for (i, v) in self.s.iter_mut().enumerate() {
            let edge = (i as f32 / fade_in as f32)
                .min((n - 1 - i) as f32 / fade_out as f32)
                .min(1.0);
            *v = *v / peak * 26_000.0 * edge;
        }
        self.s
    }
}
