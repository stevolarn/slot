use std::path::PathBuf;
use std::time::{Duration, Instant};

use shelf::frontend::Frontend;
use shelf::input::DeviceInput;
use shelf_gfx::{Compositor, FbdevSurface, Surface};
use shelf_power::{trace_first_frame, DevicePlatform};

const CARD: &str = "/mnt/sdcard";

const MIN_FRAME: Duration = Duration::from_millis(12);

const MARGIN: Duration = Duration::from_micros(1500);

struct Pacer {
    last_return: Option<Instant>,
    period: Duration,
    work: Duration,
    works: [Duration; WORK_WINDOW],
    next: usize,
}

const WORK_WINDOW: usize = 32;

impl Pacer {
    fn new() -> Self {
        Pacer {
            last_return: None,
            period: Duration::from_micros(16_760),
            work: Duration::from_millis(4),
            works: [Duration::from_millis(4); WORK_WINDOW],
            next: 0,
        }
    }

    fn wait(&self) {
        let Some(last) = self.last_return else {
            return;
        };
        let delay = self.period.saturating_sub(self.work + MARGIN);
        if let Some(left) = (last + delay).checked_duration_since(Instant::now()) {
            std::thread::sleep(left);
        }
    }

    fn swapped(&mut self, work: Duration, blocked: bool) -> bool {
        let now = Instant::now();
        let mut missed = false;
        if let Some(last) = self.last_return {
            let seen = now - last;
            missed = seen > self.period.mul_f64(1.5);
            if blocked && seen > Duration::from_millis(12) && seen < Duration::from_millis(22) {
                self.period = self.period.mul_f64(0.95) + seen.mul_f64(0.05);
            }
        }
        self.works[self.next] = work;
        self.next = (self.next + 1) % WORK_WINDOW;
        self.work = self.works.iter().copied().max().unwrap_or(work);
        self.last_return = Some(now);
        missed
    }
}

pub fn run() {
    shelf::boot_time::mark("start");
    let _ = std::thread::Builder::new()
        .name("shelf-fonts".into())
        .spawn(shelf_ui::warm_fonts);
    let audio = std::thread::Builder::new()
        .name("shelf-audio-open".into())
        .spawn(shelf::audio::opened_sink);
    let handoff = shelf::handoff::file();
    let root = std::env::var_os("SHELF_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CARD));
    let mut surface = match FbdevSurface::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("shelf: {e}");
            return;
        }
    };
    shelf::boot_time::mark("surface");
    let mut compositor = match Compositor::new(&surface) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("shelf: {e}");
            return;
        }
    };
    shelf::boot_time::mark("compositor");
    let window = surface.window_size();
    eprintln!("shelf: panel {}x{}", window.0, window.1);
    let platform = DevicePlatform::new(root.clone());
    eprintln!("shelf: {}", platform.report());
    platform.trace_boot();
    shelf::boot_time::mark("platform");
    let sink = match audio.map(|h| h.join()) {
        Ok(Ok(sink)) => sink,
        _ => shelf::audio::opened_sink(),
    };
    shelf::boot_time::mark("audio");
    let mut frontend = Frontend::boot_with(Box::new(platform), sink);
    shelf::boot_time::mark("frontend");
    frontend.upload_boot_faces(&mut compositor);
    shelf::boot_time::mark("faces");
    frontend.fit_panel(window);
    shelf::boot_time::mark("panel");
    let mut input = DeviceInput::open(&root);
    shelf::boot_time::mark("input");
    let mut drawn = false;
    let mut pacer = Pacer::new();
    loop {
        pacer.wait();
        let began = Instant::now();
        frontend.advance(&mut input);
        let line = if let Some(rom) = frontend.handed_off() {
            Some(shelf::handoff::rom_line(rom, frontend.resume_saves()))
        } else if frontend.restarting() {
            Some(shelf::handoff::REBOOT.to_string())
        } else if frontend.powering_off() {
            Some(shelf::handoff::POWER_OFF.to_string())
        } else {
            None
        };
        if let Some(line) = line {
            match &handoff {
                Some(file) => shelf::handoff::write(file, &line),
                None => println!("{line}"),
            }
            return;
        }
        if frontend.exiting() {
            return;
        }
        if frontend.restarting() {
            frontend.restart();
        }
        if frontend.powering_off() {
            frontend.poweroff();
            return;
        }
        frontend.render(&mut compositor, window);
        let swap = Instant::now();
        let work = swap - began;
        if let Err(e) = surface.swap() {
            eprintln!("shelf: {e}");
            return;
        }
        let swap_took = swap.elapsed();
        pacer.swapped(work, swap_took > Duration::from_millis(1));
        if !drawn {
            drawn = true;
            shelf::boot_time::mark("first frame");
            trace_first_frame();
        }
        if let Some(left) = MIN_FRAME.checked_sub(began.elapsed()) {
            std::thread::sleep(left);
        }
    }
}
