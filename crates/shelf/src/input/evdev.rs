use std::path::{Path, PathBuf};

use shelf_input::{Btn, RawEvent};

pub const EVENT_BYTES: usize = 24;

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;
pub const ABS_HAT0X: u16 = 0x10;
pub const ABS_HAT0Y: u16 = 0x11;
/// L2 and R2 on the Brick are analog axes, 0 (released) to 255.
pub const ABS_Z: u16 = 0x02;
pub const ABS_RZ: u16 = 0x05;

/// A trigger counts as pressed above this, and stays pressed until it
/// falls below `TRIGGER_UP`, so a wobbling axis does not chatter.
pub const TRIGGER_DOWN: i32 = 128;
pub const TRIGGER_UP: i32 = 64;

/// The D-pad hat and the L2/R2 trigger axes, turned into button events.
#[derive(Default)]
pub struct Hat {
    x: i32,
    y: i32,
    l2: bool,
    r2: bool,
}

impl Hat {
    pub fn feed(&mut self, ev: Ev) -> Vec<RawEvent> {
        let trigger = match ev.code {
            ABS_Z => Some((&mut self.l2, Btn::L2)),
            ABS_RZ => Some((&mut self.r2, Btn::R2)),
            _ => None,
        };
        if let Some((down, btn)) = trigger {
            let now = if *down {
                ev.value >= TRIGGER_UP
            } else {
                ev.value > TRIGGER_DOWN
            };
            if now == *down {
                return Vec::new();
            }
            *down = now;
            return vec![if now {
                RawEvent::Down(btn)
            } else {
                RawEvent::Up(btn)
            }];
        }
        let (held, ends) = match ev.code {
            ABS_HAT0X => (&mut self.x, [Btn::Left, Btn::Right]),
            ABS_HAT0Y => (&mut self.y, [Btn::Up, Btn::Down]),
            _ => return Vec::new(),
        };
        let (was, now) = (*held, ev.value.signum());
        if now == was {
            return Vec::new();
        }
        *held = now;
        let btn = |v: i32| ends[usize::from(v > 0)];
        let mut out = Vec::new();
        if was != 0 {
            out.push(RawEvent::Up(btn(was)));
        }
        if now != 0 {
            out.push(RawEvent::Down(btn(now)));
        }
        out
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Ev {
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

pub fn decode(bytes: &[u8]) -> Option<Ev> {
    let packet: &[u8; EVENT_BYTES] = bytes.get(..EVENT_BYTES)?.try_into().ok()?;
    Some(Ev {
        kind: u16::from_le_bytes([packet[16], packet[17]]),
        code: u16::from_le_bytes([packet[18], packet[19]]),
        value: i32::from_le_bytes([packet[20], packet[21], packet[22], packet[23]]),
    })
}

pub fn to_raw(ev: Ev) -> Option<RawEvent> {
    if ev.kind != EV_KEY {
        return None;
    }
    let btn = code_to_btn(ev.code)?;
    match ev.value {
        0 => Some(RawEvent::Up(btn)),
        1 => Some(RawEvent::Down(btn)),
        _ => None,
    }
}

/// The Brick's buttons, as NextUI's tg5040 platform numbers them: its
/// joystick buttons B, A, Y, X, L1, R1, Select, Start and Menu are these
/// codes in this order.
pub fn code_to_btn(code: u16) -> Option<Btn> {
    Some(match code {
        0x130 => Btn::B,
        0x131 => Btn::A,
        0x133 => Btn::Y,
        0x134 => Btn::X,
        0x136 => Btn::L1,
        0x137 => Btn::R1,
        0x13a => Btn::Select,
        0x13b => Btn::Start,
        0x13c => Btn::Menu,
        115 => Btn::VolUp,
        114 => Btn::VolDown,
        116 => Btn::Power,
        _ => return None,
    })
}

const WANTED: [u16; 15] = [
    0x130, 0x131, 0x132, 0x133, 0x134, 0x135, 0x136, 0x137, 0x138, 0x13a, 0x13b, 0x162, 115, 114,
    116,
];

pub use shelf_power::has_bit;

pub fn pick_devices(dev: &Path, sys: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dev) else {
        return Vec::new();
    };
    let mut nodes: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("event"))
        })
        .collect();
    nodes.sort();
    nodes.retain(|node| {
        let Some(name) = node.file_name() else {
            return false;
        };
        wanted_node(&sys.join(name).join("device"))
    });
    nodes
}

fn wanted_node(device: &Path) -> bool {
    let keys = std::fs::read_to_string(device.join("capabilities").join("key")).unwrap_or_default();
    WANTED.iter().any(|bit| has_bit(&keys, *bit))
}

pub fn device_name(sys: &Path, node: &Path) -> String {
    node.file_name()
        .and_then(|n| std::fs::read_to_string(sys.join(n).join("device/name")).ok())
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| node.display().to_string())
}
