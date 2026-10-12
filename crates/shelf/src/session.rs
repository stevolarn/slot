use std::path::PathBuf;

use shelf_input::{Action, Gestures, Millis, RawEvent};

use crate::app::App;
use crate::audio::{opened_sink, AudioSink, Ring, Sfx};
use crate::prefetch::Prefetch;

pub struct Session {
    app: App,
    sink: Box<dyn AudioSink>,
    gestures: Gestures,
    prefetch: Prefetch,
}

impl Session {
    pub fn boot(root: PathBuf) -> Self {
        Self::boot_with(root, opened_sink())
    }

    pub fn boot_with(root: PathBuf, sink: Box<dyn AudioSink>) -> Self {
        Self::with_app(App::boot(&root), sink)
    }

    pub fn with_app(app: App, sink: Box<dyn AudioSink>) -> Self {
        Session {
            app,
            sink,
            gestures: Gestures::new(),
            prefetch: Prefetch::default(),
        }
    }

    pub fn play_sfx(&mut self, sfx: Sfx) {
        let ring = self.sink.ring();
        let rate = ring.sample_rate();
        if rate == 0 {
            return;
        }
        let mut samples = sfx.render(rate);
        crate::audio::volume::apply(&mut samples, self.app.output_volume());
        ring.mix(&samples);
    }

    pub fn audio_queued(&self) -> usize {
        self.sink.ring().queued_frames()
    }

    pub fn audio_ring(&self) -> std::sync::Arc<Ring> {
        self.sink.ring()
    }

    pub fn prefetching(&self) -> Option<&std::path::Path> {
        self.prefetch.reading()
    }

    /// Whether the next update has work to do whatever the screen shows:
    /// a button held, or something the app is carrying on.
    pub fn busy(&self) -> bool {
        self.gestures.pending() || self.app.animating()
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    pub fn feed(&mut self, events: impl IntoIterator<Item = RawEvent>, now: Millis) {
        let mut actions = Vec::new();
        for ev in events {
            actions.extend(self.gestures.feed(ev, now));
        }
        actions.extend(self.gestures.tick(now));
        for action in actions {
            self.act(action);
        }
    }

    fn act(&mut self, action: Action) {
        if trace() {
            eprintln!("shelf: {action:?} in {:?}", self.app.phase());
        }
        self.app.apply(action);
    }

    pub fn update(&mut self, dt: f32) {
        self.app.update(dt);
        if let Some(sfx) = self.app.take_sfx() {
            self.play_sfx(sfx);
        }
        let now = self.app.now();
        self.prefetch.browse(self.app.browsed_rom(), now);
    }
}

pub(crate) fn trace() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SHELF_TRACE").is_some())
}
