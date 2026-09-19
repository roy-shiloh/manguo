//! Lock-free single-producer / single-consumer ring for microphone samples.
//!
//! Written by the audio-input callback, read by the granular delay node on
//! the audio-output thread. Both sides are realtime callbacks: no allocation,
//! no locks. Samples are stored as f32 bits in atomics; the producer publishes
//! its write position after the samples so the consumer never reads ahead.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::LazyLock;

/// The one microphone ring shared by the input callback and the grain node.
pub static MIC_RING: LazyLock<MicRing> = LazyLock::new(MicRing::new);

pub const MIC_RING_LEN: usize = 1 << 15; // 32768 frames, ~0.68 s at 48 kHz

pub struct MicRing {
    samples: Box<[AtomicU32]>,
    /// total frames written (monotonic)
    write_pos: AtomicU64,
}

impl MicRing {
    pub fn new() -> Self {
        let samples: Vec<AtomicU32> = (0..MIC_RING_LEN).map(|_| AtomicU32::new(0)).collect();
        Self { samples: samples.into_boxed_slice(), write_pos: AtomicU64::new(0) }
    }

    /// Producer side. Mono samples.
    pub fn push(&self, frames: &[f32]) {
        let mut pos = self.write_pos.load(Ordering::Relaxed);
        for s in frames {
            self.samples[(pos as usize) & (MIC_RING_LEN - 1)].store(s.to_bits(), Ordering::Relaxed);
            pos += 1;
        }
        self.write_pos.store(pos, Ordering::Release);
    }

    /// Consumer side: total frames written so far.
    pub fn write_pos(&self) -> u64 {
        self.write_pos.load(Ordering::Acquire)
    }

    /// Consumer side: sample at absolute frame index (wraps). Returns 0 for
    /// frames that have been overwritten or not yet written.
    #[inline]
    pub fn get(&self, abs_pos: u64, write_pos: u64) -> f32 {
        if abs_pos >= write_pos || write_pos - abs_pos > MIC_RING_LEN as u64 {
            return 0.0;
        }
        f32::from_bits(self.samples[(abs_pos as usize) & (MIC_RING_LEN - 1)].load(Ordering::Relaxed))
    }
}

impl Default for MicRing {
    fn default() -> Self {
        Self::new()
    }
}
