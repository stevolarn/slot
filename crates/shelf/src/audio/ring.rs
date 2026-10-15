//! The mixing bus the shelf's sound effects are laid onto and the output
//! device drains, a period at a time. It plays silence when nothing is laid.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

/// How many frames the bus holds for `sample_rate`: a little over a tenth of
/// a second, enough for the longest effect to be laid in one go.
pub fn ring_capacity(sample_rate: u32) -> usize {
    sample_rate as usize * 8 / 60
}

struct Inner {
    buf: Vec<i16>,
    head: usize,
    len: usize,
    pending: Vec<i16>,
    placed: usize,
    pending_at: u64,
    drained: u64,
}

pub struct Ring {
    inner: Mutex<Inner>,
    /// Told when an effect is laid, for an output waiting on silence.
    laid: Condvar,
    queued: AtomicUsize,
    capacity: AtomicUsize,
    rate: AtomicU32,
}

impl Ring {
    pub fn new(capacity_frames: usize) -> Self {
        Ring {
            inner: Mutex::new(Inner {
                buf: vec![0; capacity_frames * 2],
                head: 0,
                len: 0,
                pending: Vec::new(),
                placed: 0,
                pending_at: 0,
                drained: 0,
            }),
            laid: Condvar::new(),
            queued: AtomicUsize::new(0),
            capacity: AtomicUsize::new(capacity_frames),
            rate: AtomicU32::new(0),
        }
    }

    pub fn reopen(&self, sample_rate: u32) {
        let frames = ring_capacity(sample_rate);
        let mut i = self.lock();
        i.buf = vec![0; frames * 2];
        i.head = 0;
        i.len = 0;
        i.pending = Vec::new();
        i.placed = 0;
        i.pending_at = 0;
        i.drained = 0;
        self.queued.store(0, Ordering::Relaxed);
        self.capacity.store(frames, Ordering::Relaxed);
        self.rate.store(sample_rate, Ordering::Relaxed);
    }

    /// Lay `samples` onto the bus, to start playing from the next period,
    /// over whatever is already waiting there. An effect laid while another
    /// is still going replaces the rest of it.
    pub fn mix(&self, samples: &[i16]) {
        let mut i = self.lock();
        i.pending.clear();
        i.placed = 0;
        if i.buf.is_empty() {
            return;
        }
        i.pending_at = i.drained;
        i.pending.extend_from_slice(samples);
        lay_pending(&mut i);
        self.queued.store(i.len, Ordering::Relaxed);
        drop(i);
        self.laid.notify_all();
    }

    /// Wait, up to `most`, until an effect is laid or `stop` is set: the
    /// output, closed after a long silence, sleeps here rather than
    /// draining silence a period at a time.
    pub fn wait_for_sound(&self, stop: &AtomicBool, most: Duration) {
        let i = self.lock();
        let _ = self
            .laid
            .wait_timeout_while(i, most, |i| {
                i.len == 0 && i.placed >= i.pending.len() && !stop.load(Ordering::Relaxed)
            });
    }

    /// Wake whatever is in [`Ring::wait_for_sound`], to see `stop`.
    pub fn wake(&self) {
        let _i = self.lock();
        self.laid.notify_all();
    }

    /// Drain one output period into `out`, silence after what was laid.
    pub fn fill(&self, out: &mut [i16]) {
        let mut i = self.lock();
        lay_pending(&mut i);
        let cap = i.buf.len();
        let n = out.len().min(i.len);
        if cap > 0 {
            let first = n.min(cap - i.head);
            out[..first].copy_from_slice(&i.buf[i.head..i.head + first]);
            out[first..n].copy_from_slice(&i.buf[..n - first]);
            i.head = (i.head + n) % cap;
            i.len -= n;
            i.drained += n as u64;
        }
        out[n..].fill(0);
        self.queued.store(i.len, Ordering::Relaxed);
    }

    pub fn queued_frames(&self) -> usize {
        self.queued.load(Ordering::Relaxed) / 2
    }

    pub fn capacity_frames(&self) -> usize {
        self.capacity.load(Ordering::Relaxed)
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate.load(Ordering::Relaxed)
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn lay_pending(i: &mut Inner) {
    let cap = i.buf.len();
    if cap == 0 || i.placed >= i.pending.len() {
        return;
    }
    let Some(off) = i.pending_at.checked_sub(i.drained) else {
        i.pending.clear();
        i.placed = 0;
        return;
    };
    let off = off as usize;
    if off >= cap {
        return;
    }
    let n = (i.pending.len() - i.placed).min(cap - off);
    let len = i.len;
    for k in 0..n {
        let s = i.pending[i.placed + k];
        let at = (i.head + off + k) % cap;
        i.buf[at] = if off + k < len {
            i.buf[at].saturating_add(s)
        } else {
            s
        };
    }
    i.len = i.len.max(off + n);
    i.placed += n;
    i.pending_at += n as u64;
    if i.placed >= i.pending.len() {
        i.pending = Vec::new();
        i.placed = 0;
    }
}
