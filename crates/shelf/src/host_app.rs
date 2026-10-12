use std::time::{Duration, Instant};

use shelf::frontend::Frontend;
use shelf::input::HostInput;
use shelf_gfx::{Compositor, HostSurface, Surface};
use shelf_power::SimPlatform;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

const FRAME: Duration = Duration::from_micros(16_667);

struct Slot {
    gfx: Option<(HostSurface, Compositor)>,
    frontend: Frontend,
    input: HostInput,
}

impl Slot {
    fn new() -> Self {
        Slot {
            gfx: None,
            frontend: Frontend::boot(Box::new(SimPlatform::new())),
            input: HostInput::new(),
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
                eprintln!("slot: {e}");
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
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::Resized(size) => surface.resize(size),
            WindowEvent::RedrawRequested => {
                let began = Instant::now();
                self.frontend.render(compositor, surface.window_size());
                if let Err(e) = surface.swap() {
                    eprintln!("slot: {e}");
                    events.exit();
                    return;
                }
                surface.request_redraw();
                self.frontend.advance(&mut self.input);
                let line = if let Some(rom) = self.frontend.handed_off() {
                    Some(rom.display().to_string())
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
                if let Some(left) = FRAME.checked_sub(began.elapsed()) {
                    std::thread::sleep(left);
                }
            }
            _ => {}
        }
    }
}

pub fn run() {
    let events = match EventLoop::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    events.set_control_flow(ControlFlow::Poll);
    if let Err(e) = events.run_app(&mut Slot::new()) {
        eprintln!("slot: {e}");
    }
}
