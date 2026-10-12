use std::time::{Duration, Instant};

use shelf::frontend::Frontend;
use shelf::input::HostInput;
use shelf_gfx::{Compositor, HostSurface, Surface};
use shelf_power::SimPlatform;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

const FRAME: Duration = Duration::from_micros(16_667);

struct Slot {
    gfx: Option<(HostSurface, Compositor)>,
    frontend: Frontend,
    input: HostInput,
    input_woke: bool,
}

impl Slot {
    fn new() -> Self {
        Slot {
            gfx: None,
            frontend: Frontend::boot(Box::new(SimPlatform::new())),
            input: HostInput::new(),
            input_woke: false,
        }
    }
}

impl ApplicationHandler for Slot {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let built = HostSurface::new(events).and_then(|s| Compositor::new(&s).map(|c| (s, c)));
        let (surface, mut compositor) = match built {
            Ok(gfx) => gfx,
            Err(e) => {
                eprintln!("shelf: {e}");
                events.exit();
                return;
            }
        };
        self.frontend.upload_faces(&mut compositor);
        self.frontend.fit_panel(surface.window_size());
        self.gfx = Some((surface, compositor));
    }

    fn window_event(&mut self, events: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        self.input.on_window_event(&event);
        let Some((surface, compositor)) = self.gfx.as_mut() else {
            return;
        };
        if !matches!(event, WindowEvent::RedrawRequested) {
            // Any input may need a frame; ask for one in case the loop is
            // waiting.
            self.input_woke = true;
            surface.request_redraw();
        }
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::Resized(size) => surface.resize(size),
            WindowEvent::RedrawRequested => {
                let began = Instant::now();
                if self.frontend.render(compositor, surface.window_size()) {
                    if let Err(e) = surface.swap() {
                        eprintln!("shelf: {e}");
                        events.exit();
                        return;
                    }
                }
                self.frontend.advance(&mut self.input);
                // With nothing to draw and nothing held or loading, wake
                // on the next input or after a tick rather than spinning.
                // A frame woken by input goes on drawing: the input has only
                // just been applied.
                let woken = std::mem::take(&mut self.input_woke);
                if self.frontend.at_rest() && !woken {
                    events.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + shelf::idle::REST_TICK,
                    ));
                } else {
                    events.set_control_flow(ControlFlow::Poll);
                    surface.request_redraw();
                }
                let line = if let Some(rom) = self.frontend.handed_off() {
                    Some(shelf::handoff::rom_line(rom, self.frontend.resume_saves()))
                } else if self.frontend.restarting() {
                    Some(shelf::handoff::REBOOT.to_string())
                } else if self.frontend.powering_off() {
                    Some(shelf::handoff::POWER_OFF.to_string())
                } else {
                    None
                };
                if let Some(line) = line {
                    match shelf::handoff::file() {
                        Some(file) => shelf::handoff::write(&file, &line),
                        None => println!("{line}"),
                    }
                    events.exit();
                    return;
                }
                if self.frontend.exiting() {
                    events.exit();
                    return;
                }
                if self.frontend.restarting() {
                    self.frontend.restart();
                }
                if self.frontend.powering_off() {
                    self.frontend.poweroff();
                    events.exit();
                    return;
                }
                if !self.frontend.at_rest() || woken {
                    if let Some(left) = FRAME.checked_sub(began.elapsed()) {
                        std::thread::sleep(left);
                    }
                }
            }
            _ => {}
        }
    }

    fn new_events(&mut self, _events: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            if let Some((surface, _)) = &self.gfx {
                surface.request_redraw();
            }
        }
    }
}

pub fn run() {
    let events = match EventLoop::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("shelf: {e}");
            return;
        }
    };
    events.set_control_flow(ControlFlow::Poll);
    if let Err(e) = events.run_app(&mut Slot::new()) {
        eprintln!("shelf: {e}");
    }
}
