//! Grain delay engine: a multi-generational time-reflection device (after the
//! EarthQuaker Devices Silos) whose repeats live in a Game of Life.
//!
//! Framework-free port of the Manguo engine. The host feeds the input buffer,
//! parameters arrive as a plain [`EngineParams`] snapshot per block, and the
//! grid is shared with the editor through the atomics in [`GridShared`].
//!
//! * The input is chopped into grains one slot long (slot = Time / 16).
//! * A cell born by a click, a seed or an input stamp captures the input slice
//!   at its column's offset when the playhead passes it, and replays it one
//!   Time later.
//! * Every `Time` seconds a generation runs. Survivors replay their grain one
//!   generation older, gain multiplied by Repeats. Newborn cells inherit a
//!   neighbouring parent's grain (gliders carry grains across the grid,
//!   shifting their time slot and pitch). Dead cells drop theirs.
//! * Mode D/A/T degrades a grain by its age: Digital darkens each generation,
//!   Analog loses fidelity and self-oscillates at high Repeats, Tape saturates
//!   harder every cycle depending on input level.
//! * With Life off the grid is a static multi-tap delay whose taps decay.

use crate::life::{Life, Pattern, next_u32, H, W};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const CELLS: usize = W * H;
/// samples stored per cell grain
const GRAIN_MAX: usize = 32768;
const MAX_VOICES: usize = 96;
/// input history so a birth can capture slices from the recent past
const HIST: usize = 1 << 18;
/// grain edge fade, seconds
const FADE_S: f32 = 0.005;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GrainScale {
    #[default]
    Minor,
    Major,
    Dorian,
    Pentatonic,
}

impl GrainScale {
    pub fn from_index(i: u32) -> Self {
        match i {
            1 => GrainScale::Major,
            2 => GrainScale::Dorian,
            3 => GrainScale::Pentatonic,
            _ => GrainScale::Minor,
        }
    }

    fn intervals(&self) -> &'static [i32] {
        match self {
            GrainScale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            GrainScale::Major => &[0, 2, 4, 5, 7, 9, 11],
            GrainScale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            GrainScale::Pentatonic => &[0, 2, 4, 7, 9],
        }
    }

    fn semitones(&self, degree: i32) -> i32 {
        let iv = self.intervals();
        let n = iv.len() as i32;
        degree.div_euclid(n) * 12 + iv[degree.rem_euclid(n) as usize]
    }
}

/// Silos-style repeat character.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DelayMode {
    #[default]
    Digital,
    Analog,
    Tape,
}

impl DelayMode {
    pub fn from_index(i: u32) -> Self {
        match i {
            1 => DelayMode::Analog,
            2 => DelayMode::Tape,
            _ => DelayMode::Digital,
        }
    }
}

/// Per-block parameter snapshot (plain values, read from the host params).
#[derive(Clone, Copy, Debug)]
pub struct EngineParams {
    /// generations happen, playhead runs
    pub running: bool,
    /// Life rules on; off = static multi-tap delay
    pub life: bool,
    /// hold: no generations, no aging, no new recording
    pub freeze: bool,
    pub mode: DelayMode,
    pub scale: GrainScale,
    /// delay time = one generation, seconds
    pub time: f32,
    /// regeneration gain per generation, 0..1
    pub repeats: f32,
    /// grain length in slots, 1..4
    pub size: f32,
    /// how much of the row pitch applies, 0..1
    pub pitch: f32,
    /// probability a grain plays backwards
    pub reverse: f32,
    /// random trigger offset, fraction of a slot
    pub spray: f32,
    /// probability a live cell fires when the playhead reaches it
    pub density: f32,
    /// stereo spread by row
    pub width: f32,
    /// return filter: -1 low-pass down to 500 Hz .. +1 high-pass up to 2 kHz
    pub tone: f32,
    /// return attenuated while the input is present
    pub duck: f32,
    /// dry (0) .. wet (1), equal power
    pub mix: f32,
    /// input gain, linear
    pub input_gain: f32,
    /// input presence RMS threshold; above f32::MAX/2 means closed
    pub gate_thresh: f32,
    /// presence gate release, seconds
    pub gate_release: f32,
    /// input onsets stamp cells at the playhead (row from detected pitch)
    pub auto_stamp: bool,
}

impl Default for EngineParams {
    fn default() -> Self {
        Self {
            running: true,
            life: false,
            freeze: false,
            mode: DelayMode::Digital,
            scale: GrainScale::Minor,
            time: 0.5,
            repeats: 0.6,
            size: 2.0,
            pitch: 1.0,
            reverse: 0.0,
            spray: 0.0,
            density: 1.0,
            width: 0.5,
            tone: 0.0,
            duck: 0.0,
            mix: 0.5,
            input_gain: 1.0,
            gate_thresh: 0.03,
            gate_release: 0.3,
            auto_stamp: true,
        }
    }
}

/// Grid and meters shared between the audio thread and the editor.
///
/// `steps[x]` bit `y` is the cell in slot `x`, row `y`. The editor writes
/// clicks/seeds into `steps`; the engine diffs them against what it last
/// published, captures input for births, and publishes the grid back after
/// every generation.
pub struct GridShared {
    pub steps: [AtomicU32; W],
    pub current_slot: AtomicU32,
    pub generation: AtomicU32,
    pub population: AtomicU32,
    /// peak repeat level since the editor last took it, f32 bits
    pub grain_level: AtomicU32,
    /// smoothed input RMS, f32 bits
    pub input_level: AtomicU32,
    pub present: AtomicBool,
    /// longest process call so far, microseconds
    pub max_render_us: AtomicU32,
    /// editor asks for a random pattern at the playhead (count)
    pub seed_request: AtomicU32,
    /// editor asks for a clear
    pub clear_request: AtomicBool,
}

impl Default for GridShared {
    fn default() -> Self {
        Self {
            steps: std::array::from_fn(|_| AtomicU32::new(0)),
            current_slot: AtomicU32::new(0),
            generation: AtomicU32::new(0),
            population: AtomicU32::new(0),
            grain_level: AtomicU32::new(0),
            input_level: AtomicU32::new(0),
            present: AtomicBool::new(false),
            max_render_us: AtomicU32::new(0),
            seed_request: AtomicU32::new(0),
            clear_request: AtomicBool::new(false),
        }
    }
}

impl GridShared {
    pub fn steps(&self) -> [u32; W] {
        std::array::from_fn(|x| self.steps[x].load(Ordering::Relaxed) & 0xFFFF)
    }

    pub fn set_steps(&self, steps: &[u32; W]) {
        for x in 0..W {
            self.steps[x].store(steps[x] & 0xFFFF, Ordering::Relaxed);
        }
    }

    pub fn toggle(&self, x: usize, y: usize) {
        self.steps[x % W].fetch_xor(1 << (y % H), Ordering::Relaxed);
    }

    pub fn take_grain_level(&self) -> f32 {
        f32::from_bits(self.grain_level.swap(0, Ordering::Relaxed))
    }

    pub fn input_level(&self) -> f32 {
        f32::from_bits(self.input_level.load(Ordering::Relaxed))
    }
}

#[derive(Clone, Copy, Default)]
struct Cell {
    alive: bool,
    /// born, waiting for the playhead to pass its column and record the slice
    pending: bool,
    /// generations since capture
    age: u32,
    gain: f32,
    /// valid samples in the grain
    len: usize,
    /// per-cell wow phase (tape)
    phase: f32,
}

#[derive(Clone, Copy, Default)]
struct Voice {
    active: bool,
    cell: usize,
    pos: f64,
    rate: f64,
    len: usize,
    reverse: bool,
    /// spray: samples to wait before the grain starts
    delay: u32,
    gain: f32,
    gain_l: f32,
    gain_r: f32,
    mode: u8,
    lp: f32,
    lp_a: f32,
    hold: f32,
    hold_n: u8,
    dec: u8,
    noise: f32,
    drive: f32,
    wow_phase: f32,
    wow_depth: f32,
    rng: u32,
}

pub struct Engine {
    sr: f32,
    fade: usize,
    p: EngineParams,
    grains: Vec<f32>,
    cells: Vec<Cell>,
    grid: Life,
    /// what we last published to `GridShared::steps`
    published: [u32; W],
    parents: [[u8; H]; W],
    voices: Vec<Voice>,
    samples_to_slot: u32,
    slot: usize,
    was_running: bool,
    hist: Vec<f32>,
    /// absolute frames written to `hist`
    write: u64,
    input_level: f32,
    present: bool,
    duck_env: f32,
    gate_env: f32,
    tone_lp: [f32; 2],
    tone_hp: [f32; 2],
    seed: u32,
    /// pitch detector scratch (mono, newest input)
    pitch_buf: Vec<f32>,
    /// onset waiting for enough recorded input to detect its pitch
    onset_pending: bool,
    onset_at: u64,
    onset_rms: f32,
}

#[inline]
fn cell_index(x: usize, y: usize) -> usize {
    x * H + y
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(8000.0);
        Self {
            sr,
            fade: (FADE_S * sr) as usize,
            p: EngineParams::default(),
            grains: vec![0.0; CELLS * GRAIN_MAX],
            cells: vec![Cell::default(); CELLS],
            grid: Life::default(),
            published: [0; W],
            parents: [[u8::MAX; H]; W],
            voices: vec![Voice::default(); MAX_VOICES],
            samples_to_slot: 0,
            slot: W - 1,
            was_running: false,
            hist: vec![0.0; HIST],
            write: 0,
            input_level: 0.0,
            present: false,
            duck_env: 0.0,
            gate_env: 0.0,
            tone_lp: [0.0; 2],
            tone_hp: [0.0; 2],
            seed: 0x9E37_79B9,
            pitch_buf: vec![0.0; 2048],
            onset_pending: false,
            onset_at: 0,
            onset_rms: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sr = sample_rate.max(8000.0);
        self.fade = (FADE_S * self.sr) as usize;
    }

    /// Reset the audio state, keeping the grid pattern (cells wait to record).
    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.active = false;
        }
        self.hist.iter_mut().for_each(|s| *s = 0.0);
        self.tone_lp = [0.0; 2];
        self.tone_hp = [0.0; 2];
        self.duck_env = 0.0;
        self.gate_env = 0.0;
        self.present = false;
        self.onset_pending = false;
    }

    fn rnd(&mut self) -> f32 {
        (next_u32(&mut self.seed) >> 8) as f32 / (1u32 << 24) as f32
    }

    fn slot_len(&self) -> usize {
        let time = self.p.time.clamp(0.05, 4.0);
        ((self.sr * time) as usize / W).max(64)
    }

    fn grain_len(&self) -> usize {
        let size = self.p.size.clamp(1.0, 4.0);
        ((self.slot_len() as f32 * size) as usize).clamp(self.fade * 2, GRAIN_MAX)
    }

    fn birth_pending(&mut self, x: usize, y: usize) {
        let ci = cell_index(x, y);
        let phase = self.rnd();
        self.cells[ci] = Cell { alive: true, pending: true, age: 0, gain: 1.0, len: 0, phase };
    }

    /// Record the last `grain_len` input samples into cell (x, y).
    fn capture(&mut self, x: usize, y: usize) {
        let len = self.grain_len();
        let start = self.write.saturating_sub(len as u64);
        let ci = cell_index(x, y);
        let base = ci * GRAIN_MAX;
        for i in 0..len {
            let p = start + i as u64;
            let s = if self.write - p <= HIST as u64 && p < self.write {
                self.hist[(p as usize) & (HIST - 1)]
            } else {
                0.0
            };
            self.grains[base + i] = s;
        }
        let c = &mut self.cells[ci];
        c.pending = false;
        c.len = len;
    }

    fn record_pending(&mut self, x: usize) {
        for y in 0..H {
            let ci = cell_index(x, y);
            if self.cells[ci].alive && self.cells[ci].pending {
                self.capture(x, y);
            }
        }
    }

    fn inherit(&mut self, x: usize, y: usize, parent: usize) {
        let ci = cell_index(x, y);
        let p = self.cells[parent];
        if p.pending || p.len == 0 {
            self.birth_pending(x, y);
            return;
        }
        let (pb, cb) = (parent * GRAIN_MAX, ci * GRAIN_MAX);
        let len = p.len;
        if pb < cb {
            let (a, b) = self.grains.split_at_mut(cb);
            b[..len].copy_from_slice(&a[pb..pb + len]);
        } else {
            let (a, b) = self.grains.split_at_mut(pb);
            a[cb..cb + len].copy_from_slice(&b[..len]);
        }
        let repeats = self.regen_gain();
        self.cells[ci] = Cell { alive: true, pending: false, age: p.age + 1, gain: p.gain * repeats, len, phase: p.phase };
    }

    fn kill(&mut self, x: usize, y: usize) {
        self.cells[cell_index(x, y)] = Cell::default();
    }

    /// Repeats knob to per-generation gain. Digital never self-oscillates;
    /// Analog and Tape pass unity around "3 o'clock".
    fn regen_gain(&self) -> f32 {
        let r = self.p.repeats.clamp(0.0, 1.0);
        match self.p.mode {
            DelayMode::Digital => r * 0.98,
            _ => {
                if r < 0.75 {
                    r * 1.3
                } else {
                    0.975 + (r - 0.75) * 0.4
                }
            }
        }
    }

    /// Editor edits: `steps` differing from what we published means a click,
    /// a seed or a stamp. Births capture input; deaths drop their grain.
    fn apply_edits(&mut self, shared: &GridShared) {
        if shared.clear_request.swap(false, Ordering::Relaxed) {
            shared.set_steps(&[0; W]);
        }
        let seeds = shared.seed_request.swap(0, Ordering::Relaxed);
        if seeds > 0 {
            let mut grid = Life::from_steps(&shared.steps());
            for _ in 0..seeds.min(4) {
                let pattern = Pattern::pick(&mut self.seed);
                let x = (self.slot + 1 + (next_u32(&mut self.seed) % 3) as usize) % W;
                let y = (next_u32(&mut self.seed) % H as u32) as usize;
                grid.stamp(pattern, x, y, &mut self.seed);
            }
            shared.set_steps(&grid.to_steps());
        }
        let wanted = shared.steps();
        if wanted == self.published {
            return;
        }
        for x in 0..W {
            let was = self.published[x];
            let now = wanted[x];
            if was == now {
                continue;
            }
            for y in 0..H {
                let bit = 1u32 << y;
                match (was & bit != 0, now & bit != 0) {
                    (false, true) => {
                        self.grid.set(x, y, true);
                        self.birth_pending(x, y);
                    }
                    (true, false) => {
                        self.grid.set(x, y, false);
                        self.kill(x, y);
                    }
                    _ => {}
                }
            }
        }
        self.published = wanted;
        let pop = wanted.iter().map(|c| c.count_ones()).sum::<u32>();
        shared.population.store(pop, Ordering::Relaxed);
    }

    fn publish(&mut self, shared: &GridShared) {
        let steps = self.grid.to_steps();
        let mut pop = 0u32;
        for x in 0..W {
            shared.steps[x].store(steps[x], Ordering::Relaxed);
            self.published[x] = steps[x];
            pop += steps[x].count_ones();
        }
        shared.population.store(pop, Ordering::Relaxed);
    }

    /// One delay generation.
    fn generation(&mut self, shared: &GridShared) {
        let repeats = self.regen_gain();
        if self.p.life {
            let next = self.grid.step_with_parents(&mut self.parents);
            for x in 0..W {
                for y in 0..H {
                    if !self.grid.get(x, y) && next.get(x, y) {
                        let p = self.parents[x][y];
                        if p != u8::MAX && self.cells[p as usize].alive {
                            self.inherit(x, y, p as usize);
                        } else {
                            self.birth_pending(x, y);
                        }
                    }
                }
            }
            for x in 0..W {
                for y in 0..H {
                    match (self.grid.get(x, y), next.get(x, y)) {
                        (true, true) => {
                            let c = &mut self.cells[cell_index(x, y)];
                            if !c.pending {
                                c.age += 1;
                                c.gain *= repeats;
                            }
                        }
                        (true, false) => self.kill(x, y),
                        _ => {}
                    }
                }
            }
            self.grid = next;
        } else {
            for x in 0..W {
                for y in 0..H {
                    if self.grid.get(x, y) {
                        let c = &mut self.cells[cell_index(x, y)];
                        if !c.pending {
                            c.age += 1;
                            c.gain *= repeats;
                        }
                    }
                }
            }
        }
        for x in 0..W {
            for y in 0..H {
                let ci = cell_index(x, y);
                if self.grid.get(x, y) && self.cells[ci].gain < 0.002 {
                    self.grid.set(x, y, false);
                    self.kill(x, y);
                }
            }
        }
        shared.generation.fetch_add(1, Ordering::Relaxed);
        self.publish(shared);
    }

    /// Start of slot `x`: every live cell in the column replays its grain.
    fn trigger_slot(&mut self, x: usize) {
        let scale = self.p.scale;
        let mode = self.p.mode as u8;
        let pitch = self.p.pitch.clamp(0.0, 1.0);
        let reverse = self.p.reverse.clamp(0.0, 1.0);
        let spray = self.p.spray.clamp(0.0, 1.0);
        let density = self.p.density.clamp(0.0, 1.0);
        let width = self.p.width.clamp(0.0, 1.0);
        let slot_len = self.slot_len() as f32;
        let column = self.grid.cells[x];
        if column == 0 {
            return;
        }
        for y in 0..H {
            if column & (1 << y) == 0 {
                continue;
            }
            let ci = cell_index(x, y);
            let c = self.cells[ci];
            if !c.alive || c.pending || c.len == 0 {
                continue;
            }
            if density < 1.0 && self.rnd() >= density {
                continue;
            }
            let degree = 8 - y as i32;
            let semis = scale.semitones(degree) as f32 * pitch;
            let rate = 2f64.powf(semis as f64 / 12.0);
            let is_reverse = reverse > 0.0 && self.rnd() < reverse;
            let delay = if spray > 0.0 { (self.rnd() * spray * slot_len) as u32 } else { 0 };
            let pan = ((y as f32 / (H - 1) as f32) * 2.0 - 1.0) * width;
            let gl = ((1.0 - pan) * 0.5).sqrt();
            let gr = ((1.0 + pan) * 0.5).sqrt();
            let level = self.input_level;
            let age = c.age as f32;
            let (cutoff, dec, noise, drive, wow_depth) = match mode {
                0 => ((8000.0 * 0.7f32.powf(age)).max(400.0), 1u8, 0.0, 1.0, 0.0),
                1 => ((4500.0 * 0.85f32.powf(age)).max(400.0), (1 + (c.age as u8) / 2).min(8), 0.004 * age.min(6.0), 1.0, 0.0),
                _ => (
                    (3500.0 * 0.92f32.powf(age)).max(500.0),
                    1,
                    0.0,
                    1.0 + (1.0 + 8.0 * level.min(0.5)) * age.min(10.0),
                    0.003 * (1.0 + age * 0.3).min(3.0),
                ),
            };
            let lp_a = (1.0 - (-std::f32::consts::TAU * cutoff / self.sr).exp()).min(1.0);
            let rng = self.seed ^ (ci as u32).wrapping_mul(2654435761);
            if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
                *v = Voice {
                    active: true,
                    cell: ci,
                    pos: 0.0,
                    rate,
                    len: c.len,
                    reverse: is_reverse,
                    delay,
                    gain: c.gain.min(1.5),
                    gain_l: gl,
                    gain_r: gr,
                    mode,
                    lp: 0.0,
                    lp_a,
                    hold: 0.0,
                    hold_n: 0,
                    dec,
                    noise,
                    drive,
                    wow_phase: c.phase,
                    wow_depth,
                    rng: rng | 1,
                };
            }
        }
    }

    #[inline]
    fn degrade(v: &mut Voice, s: f32) -> f32 {
        match v.mode {
            0 => {
                v.lp += (s - v.lp) * v.lp_a;
                v.lp
            }
            1 => {
                if v.hold_n == 0 {
                    v.hold = s;
                }
                v.hold_n = (v.hold_n + 1) % v.dec.max(1);
                v.rng ^= v.rng << 13;
                v.rng ^= v.rng >> 17;
                v.rng ^= v.rng << 5;
                let n = (v.rng >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
                let x = v.hold + n * v.noise * (v.hold.abs() * 20.0).min(1.0);
                v.lp += (x - v.lp) * v.lp_a;
                (v.lp * 1.2).tanh()
            }
            _ => {
                v.lp += (s - v.lp) * v.lp_a;
                (v.lp * v.drive).tanh() / v.drive.sqrt()
            }
        }
    }

    /// Normalized autocorrelation over the newest input, lags 80..1000 Hz.
    /// Returns the row (0 = top) for the detected pitch, if any.
    fn detect_row(&mut self) -> Option<usize> {
        let n = self.pitch_buf.len().min(self.write as usize);
        if n < 512 {
            return None;
        }
        let start = self.write - n as u64;
        for i in 0..n {
            self.pitch_buf[i] = self.hist[((start + i as u64) as usize) & (HIST - 1)];
        }
        let buf = &self.pitch_buf[..n];
        let min_lag = (self.sr / 1000.0) as usize;
        let max_lag = ((self.sr / 80.0) as usize).min(n / 2);
        let span = n - max_lag;
        let mut e0 = 0.0f32;
        for i in 0..span {
            e0 += buf[i] * buf[i];
        }
        let mut best = 0.0f32;
        let mut best_lag = 0usize;
        let mut lag = min_lag;
        while lag < max_lag {
            let mut acc = 0.0f32;
            let mut e1 = 0.0f32;
            for i in 0..span {
                acc += buf[i] * buf[i + lag];
                e1 += buf[i + lag] * buf[i + lag];
            }
            let r = acc / (e0 * e1).sqrt().max(1e-9);
            if r > best {
                best = r;
                best_lag = lag;
            }
            lag += 1;
        }
        if best > 0.6 && best_lag > 0 {
            let hz = self.sr / best_lag as f32;
            let midi = (69.0 + 12.0 * (hz / 440.0).log2()).round().clamp(36.0, 84.0) as u8;
            Some(((midi.saturating_sub(36)) as usize * 16 / 49).min(15))
        } else {
            None
        }
    }

    /// Process one block. `input` and `output` are channel slices of equal
    /// length; mono inputs are allowed. The input is summed to mono for
    /// recording, the dry signal passes per channel.
    pub fn process(&mut self, params: &EngineParams, shared: &GridShared, input: &[&[f32]], output: &mut [&mut [f32]]) {
        let start = std::time::Instant::now();
        self.p = *params;
        let frames = output.get(0).map(|c| c.len()).unwrap_or(0);
        let channels = output.len();
        if frames == 0 || channels == 0 {
            return;
        }
        let in_ch = input.len().max(1);
        let in_gain = self.p.input_gain.max(0.0);

        // 1. input into the history (gated below, per sample), presence on the raw level
        let mut sum = 0.0f32;
        for i in 0..frames {
            let mut m = 0.0f32;
            for c in 0..input.len() {
                m += input[c][i];
            }
            let m = m / in_ch as f32 * in_gain;
            sum += m * m;
        }
        let rms = (sum / frames as f32).sqrt();
        self.input_level += (rms - self.input_level) * 0.2;
        shared.input_level.store(self.input_level.to_bits(), Ordering::Relaxed);

        let th = self.p.gate_thresh.max(1e-4);
        let was_present = self.present;
        self.present = if self.present { rms > th * 0.5 } else { rms > th };
        shared.present.store(self.present, Ordering::Relaxed);
        let target = if self.present { 1.0 } else { 0.0 };
        let sr = self.sr;
        let attack_a = 1.0 - (-1.0 / (0.005 * sr)).exp();
        let duck_a = if self.present { attack_a } else { 1.0 - (-1.0 / (0.2 * sr)).exp() };
        let gate_rel = self.p.gate_release.clamp(0.03, 2.0);
        let gate_a = if self.present { attack_a } else { 1.0 - (-1.0 / (gate_rel * sr)).exp() };

        // 2. editor edits
        self.apply_edits(shared);

        // 3. clock
        let running = self.p.running;
        if running && !self.was_running {
            self.slot = W - 1;
            self.samples_to_slot = 0;
        }
        self.was_running = running;
        let freeze = self.p.freeze;

        let mix = self.p.mix.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
        let dry = mix.cos();
        let wet = mix.sin();
        let duck = self.p.duck.clamp(0.0, 1.0);
        let dt = 1.0 / sr;
        let tone = self.p.tone.clamp(-1.0, 1.0);
        let lp_cut = 16000.0 * (500.0f32 / 16000.0).powf((-tone).max(0.0));
        let hp_cut = 30.0 * (2000.0f32 / 30.0).powf(tone.max(0.0));
        let lp_a = 1.0 - (-std::f32::consts::TAU * lp_cut / sr).exp();
        let hp_a = 1.0 - (-std::f32::consts::TAU * hp_cut / sr).exp();
        let fade = self.fade.max(1) as f32;

        // 4. onset: remember it; the pitch is read once ~40 ms of the new
        // input has been recorded (the burst that opened the gate is still
        // ahead of the history here)
        if self.p.auto_stamp && self.present && !was_present && running && !freeze {
            self.onset_pending = true;
            self.onset_at = self.write;
            self.onset_rms = rms;
        }
        if !self.p.auto_stamp || !running || freeze {
            self.onset_pending = false;
        }

        let mut peak = 0.0f32;
        for i in 0..frames {
            if running {
                if self.samples_to_slot == 0 {
                    if !freeze {
                        self.record_pending(self.slot);
                    }
                    self.slot = (self.slot + 1) % W;
                    if self.slot == 0 && !freeze {
                        self.generation(shared);
                    }
                    self.samples_to_slot = self.slot_len() as u32;
                    self.trigger_slot(self.slot);
                    shared.current_slot.store(self.slot as u32, Ordering::Relaxed);
                }
                self.samples_to_slot -= 1;
            }

            self.duck_env += (target - self.duck_env) * duck_a;
            self.gate_env += (target - self.gate_env) * gate_a;

            // gated mono input into the grain history
            let mut m = 0.0f32;
            for c in 0..input.len() {
                m += input[c][i];
            }
            let m = m / in_ch as f32 * in_gain * self.gate_env;
            self.hist[(self.write as usize) & (HIST - 1)] = m;
            self.write += 1;

            let mut gl = 0.0f32;
            let mut gr = 0.0f32;
            for v in self.voices.iter_mut() {
                if !v.active {
                    continue;
                }
                if v.delay > 0 {
                    v.delay -= 1;
                    continue;
                }
                let base = v.cell * GRAIN_MAX;
                let pi = v.pos.floor() as usize;
                if pi + 1 >= v.len {
                    v.active = false;
                    continue;
                }
                let rp = if v.reverse { (v.len - 1) as f64 - v.pos } else { v.pos };
                let ip = rp.floor();
                let f = (rp - ip) as f32;
                let i0 = (ip as usize).min(v.len - 2);
                let s = self.grains[base + i0] * (1.0 - f) + self.grains[base + i0 + 1] * f;
                let edge_in = (pi as f32 / fade).min(1.0);
                let edge_out = ((v.len - pi) as f32 / fade).min(1.0);
                let win = edge_in * edge_out;
                let s = Self::degrade(v, s) * win * v.gain;
                gl += s * v.gain_l;
                gr += s * v.gain_r;
                let wow = if v.wow_depth > 0.0 {
                    v.wow_phase = (v.wow_phase + dt * 0.9).fract();
                    let ph = v.wow_phase * 2.0 - 1.0;
                    1.0 + v.wow_depth * (ph * (1.0 - ph.abs()) * 4.0)
                } else {
                    1.0
                };
                v.pos += v.rate * wow as f64;
            }

            let ret_gain = wet * (1.0 - duck * self.duck_env);
            let mut ret = [gl, gr];
            for c in 0..2 {
                self.tone_lp[c] += (ret[c] - self.tone_lp[c]) * lp_a;
                let x = self.tone_lp[c];
                self.tone_hp[c] += (x - self.tone_hp[c]) * hp_a;
                ret[c] = ((x - self.tone_hp[c]) * ret_gain).clamp(-1.5, 1.5);
            }
            peak = peak.max(ret[0].abs()).max(ret[1].abs());
            for c in 0..channels {
                let d = input.get(c).or(input.get(0)).map(|ch| ch[i]).unwrap_or(0.0) * in_gain * self.gate_env;
                let grain = if c == 0 { ret[0] } else { ret[1] };
                output[c][i] = d * dry + grain;
            }
        }
        if peak > f32::from_bits(shared.grain_level.load(Ordering::Relaxed)) {
            shared.grain_level.store(peak.to_bits(), Ordering::Relaxed);
        }

        // 5. resolve a pending onset: stamp the grid at the playhead, row from pitch
        if self.onset_pending && self.write >= self.onset_at + 2048 {
            self.onset_pending = false;
            if let Some(row) = self.detect_row() {
                let count = if self.onset_rms > th * 2.0 { 2 } else { 1 };
                let mut grid = Life::from_steps(&shared.steps());
                let x = (self.slot + 1) % W;
                for k in 0..count {
                    grid.set((x + k) % W, row, true);
                }
                shared.set_steps(&grid.to_steps());
                self.apply_edits(shared);
            }
        }
        let us = start.elapsed().as_micros().min(u32::MAX as u128) as u32;
        shared.max_render_us.fetch_max(us, Ordering::Relaxed);
    }
}
