//! Grain delay: a multi-generational time-reflection device (after the
//! EarthQuaker Devices Silos) whose repeats live in a Game of Life.
//!
//! * The microphone is chopped into grains one slot long (slot = Time / 16).
//! * A cell that is born by a voice stamp or a click captures the input slice
//!   at its column's offset, so a pattern spanning several columns holds the
//!   consecutive slices of what was just sung.
//! * Every `Time` seconds a generation runs. Survivors replay their grain,
//!   one generation older, gain multiplied by Repeats. Newborn cells inherit
//!   a neighbouring parent's grain, so gliders carry grains across the grid,
//!   shifting their time slot (column) and pitch (row). Dead cells drop theirs.
//! * Mode D/A/T degrades a grain by its age like the Silos modes: Digital
//!   darkens each generation, Analog loses fidelity and self-oscillates at
//!   high Repeats, Tape saturates harder every cycle depending on input level.
//! * With Life off the grid is a static multi-tap delay whose taps decay.
//! * Performance controls follow the usual granular-delay set: Freeze (hold the
//!   grid and the grains), Size, Pitch amount, Reverse probability, Spray,
//!   Density, Width, a Tone filter and Duck on the return, a Dry/Wet mix, and
//!   a voice-presence Gate on the synth.
//!
//! The Life grid is owned by the audio thread. The UI edits it by writing the
//! `steps` atomics (clicks, seeds, voice stamps); the node diffs them against
//! its last published grid, capturing fresh input for births, and publishes
//! the grid back after every generation.

use crate::life::{Life, H, W};
use crate::makepad_audio_graph::*;
use crate::makepad_platform::live_atomic::*;
use crate::makepad_platform::*;
use crate::mic_ring::MIC_RING;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

live_design! {
    pub GranularDelay = {{GranularDelay}} {
        settings: {}
    }
}

const SR: f32 = 48000.0;
const CELLS: usize = W * H;
/// samples stored per cell grain (341 ms at 48 kHz)
const GRAIN_MAX: usize = 16384;
const MAX_VOICES: usize = 96;
const FADE: usize = 240; // 5 ms grain edges

#[derive(Live, LiveHook, PartialEq, LiveAtomic, Debug, LiveRead)]
pub enum GrainScale {
    #[pick]
    Minor,
    Major,
    Dorian,
    Pentatonic,
}

impl GrainScale {
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
#[derive(Live, LiveHook, PartialEq, LiveAtomic, Debug, LiveRead, Clone, Copy)]
pub enum DelayMode {
    #[pick]
    Digital,
    Analog,
    Tape,
}

#[derive(Live, LiveHook, LiveRegister, LiveAtomic, Debug, LiveRead)]
#[live_ignore]
pub struct GranularSettings {
    /// the Life grid: column = time slot within one delay period, bit = row
    /// (row 0 is the top / highest pitch). Written by the UI, published by the node.
    #[live]
    pub steps: [u32a; 16],
    /// delay running (generations happen); on at launch so the grid is live
    #[live(true)]
    pub running: boola,
    /// Life rules on; off = static multi-tap delay
    #[live(false)]
    pub life: boola,
    #[live]
    pub mode: U32A<DelayMode>,
    /// delay time = one generation, seconds
    #[live(0.5)]
    pub time: f32a,
    /// regeneration gain per generation
    #[live(0.6)]
    pub repeats: f32a,
    /// hold: no generations, no aging, no new recording
    #[live(false)]
    pub freeze: boola,
    /// grain length in slots (1..4)
    #[live(2.0)]
    pub size: f32a,
    #[live]
    pub scale: U32A<GrainScale>,
    /// how much of the row pitch applies (0 = unison delay)
    #[live(1.0)]
    pub pitch: f32a,
    /// probability a grain plays backwards
    #[live(0.0)]
    pub reverse: f32a,
    /// random trigger offset, fraction of a slot
    #[live(0.0)]
    pub spray: f32a,
    /// probability a live cell fires when the playhead reaches it
    #[live(1.0)]
    pub density: f32a,
    /// stereo spread by row
    #[live(0.5)]
    pub width: f32a,
    /// return filter: negative = low-pass down to 500 Hz, positive = high-pass up to 2 kHz
    #[live(0.0)]
    pub tone: f32a,
    /// return attenuated while the voice is present
    #[live(0.0)]
    pub duck: f32a,
    /// voice dry (0) .. repeats wet (1), equal power
    #[live(0.9)]
    pub mix: f32a,
    #[live(1.0)]
    pub synth_gain: f32a,
    /// synth level follows voice presence
    #[live(false)]
    pub gate: boola,
    /// gate release, seconds
    #[live(0.3)]
    pub gate_release: f32a,
    /// voice presence threshold (input RMS), written from Voice > Sensitivity
    #[live(0.03)]
    pub gate_thresh: f32a,
}

/// Published by the audio thread for the UI.
#[derive(Default)]
pub struct GranularShared {
    pub current_slot: AtomicU32,
    pub generation: AtomicU32,
    /// peak repeat level since the UI last read it, f32 bits
    pub grain_level: AtomicU32,
    /// number of live cells
    pub population: AtomicU32,
    /// longest render callback so far, microseconds
    pub max_render_us: AtomicU32,
    /// frames where the mic ring had nothing new (input starving the output)
    pub mic_underruns: AtomicU32,
    /// dev: set to 1 to stop the WAV recording and write it out (2 = written)
    pub record_stop: AtomicU32,
    /// frames per render callback
    pub frames: AtomicU32,
    /// dev: QA step marker, stamped into the recording timeline
    pub marker: AtomicU32,
}

#[derive(Live)]
pub struct GranularDelay {
    #[live]
    pub settings: Arc<GranularSettings>,
    #[rust(Arc::new(GranularShared::default()))]
    pub shared: Arc<GranularShared>,
    #[rust(AudioComponentRef::new(cx))]
    synth: AudioComponentRef,
}

impl LiveRegister for GranularDelay {
    fn live_register(cx: &mut Cx) {
        register_audio_component!(cx, GranularDelay)
    }
}

impl LiveHook for GranularDelay {
    fn skip_apply(&mut self, _cx: &mut Cx, apply: &mut Apply, index: usize, nodes: &[LiveNode]) -> Option<usize> {
        if let ApplyFrom::UpdateFromDoc { .. } = apply.from {
            return Some(nodes.skip_node(index));
        }
        None
    }

    fn apply_value_instance(&mut self, cx: &mut Cx, apply: &mut Apply, index: usize, nodes: &[LiveNode]) -> usize {
        self.synth.apply(cx, apply, index, nodes)
    }
}

impl AudioComponent for GranularDelay {
    fn get_graph_node(&mut self, cx: &mut Cx) -> Box<dyn AudioGraphNode + Send> {
        let synth = self.synth.as_mut().map(|s| s.get_graph_node(cx));
        Box::new(Node {
            settings: self.settings.clone(),
            shared: self.shared.clone(),
            synth,
            synth_buf: AudioBuffer::default(),
            grains: vec![0.0; CELLS * GRAIN_MAX],
            cells: [Cell::default(); CELLS],
            grid: Life::default(),
            published: [0u32; W],
            parents: [[u8::MAX; H]; W],
            voices: [Voice::default(); MAX_VOICES],
            samples_to_slot: 0,
            slot: W - 1,
            was_running: false,
            mic_read: None,
            mic_hist: vec![0.0; MIC_HIST],
            mic_write: 0,
            input_level: 0.0,
            present: false,
            duck_env: 0.0,
            gate_env: 0.0,
            tone_lp: [0.0; 2],
            tone_hp: [0.0; 2],
            seed: 0x9E37_79B9,
            rec: std::env::var("MANGUO_RECORD_WAV").ok().map(|dir| Recorder::new(dir, 90)),
            debug: std::env::var("MANGUO_LOG_GRAINS").is_ok(),
        })
    }

    fn handle_event_with(&mut self, cx: &mut Cx, event: &Event, dispatch_action: &mut dyn FnMut(&mut Cx, AudioComponentAction)) {
        if let Some(synth) = self.synth.as_mut() {
            synth.handle_event_with(cx, event, dispatch_action)
        }
    }

    fn audio_query<'a>(&'a mut self, query: &AudioQuery, callback: &mut Option<AudioQueryCb>) -> AudioResult<'a> {
        self.synth.audio_query(query, callback)
    }
}

/// local mic history so a birth can capture slices from the recent past
const MIC_HIST: usize = 1 << 17; // 2.7 s

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
    /// read position in the cell grain, fractional
    pos: f64,
    rate: f64,
    len: usize,
    /// plays the grain backwards
    reverse: bool,
    /// spray: samples to wait before the grain starts
    delay: u32,
    gain: f32,
    gain_l: f32,
    gain_r: f32,
    mode: u8,
    /// one-pole low-pass state and coefficient (fixed per voice: age is fixed)
    lp: f32,
    lp_a: f32,
    /// analog decimation: hold value, counter, factor
    hold: f32,
    hold_n: u8,
    dec: u8,
    /// analog hiss amount, tape drive
    noise: f32,
    drive: f32,
    wow_phase: f32,
    wow_depth: f32,
    /// tiny xorshift for hiss
    rng: u32,
}

struct Node {
    settings: Arc<GranularSettings>,
    shared: Arc<GranularShared>,
    synth: Option<Box<dyn AudioGraphNode + Send>>,
    synth_buf: AudioBuffer,
    /// CELLS x GRAIN_MAX sample storage
    grains: Vec<f32>,
    cells: [Cell; CELLS],
    grid: Life,
    /// what we last wrote to `settings.steps`
    published: [u32; W],
    parents: [[u8; H]; W],
    voices: [Voice; MAX_VOICES],
    samples_to_slot: u32,
    slot: usize,
    was_running: bool,
    mic_read: Option<u64>,
    mic_hist: Vec<f32>,
    /// absolute frames written to mic_hist
    mic_write: u64,
    /// smoothed input RMS
    input_level: f32,
    /// voice presence (hysteresis around gate_thresh) and its envelopes
    present: bool,
    duck_env: f32,
    gate_env: f32,
    /// return filter states, L/R
    tone_lp: [f32; 2],
    tone_hp: [f32; 2],
    seed: u32,
    rec: Option<Recorder>,
    /// dev: MANGUO_LOG_GRAINS prints births/deaths/generations (not realtime-safe)
    debug: bool,
}

/// Dev/QA: records input (mono), repeats (stereo) and output (stereo) stems
/// into memory and writes them as 32-bit float WAVs when asked to stop.
struct Recorder {
    dir: String,
    input: Vec<f32>,
    grains: Vec<f32>,
    output: Vec<f32>,
    max_frames: usize,
    done: bool,
    markers: Vec<(u32, usize)>,
    last_marker: u32,
}

impl Recorder {
    fn new(dir: String, secs: usize) -> Self {
        let max_frames = secs * SR as usize;
        Self {
            dir,
            input: Vec::with_capacity(max_frames),
            grains: Vec::with_capacity(max_frames * 2),
            output: Vec::with_capacity(max_frames * 2),
            max_frames,
            done: false,
            markers: Vec::with_capacity(64),
            last_marker: 0,
        }
    }

    fn full(&self) -> bool {
        self.input.len() >= self.max_frames
    }

    fn write(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        let _ = std::fs::create_dir_all(&self.dir);
        write_wav(&format!("{}/input.wav", self.dir), &self.input, 1);
        write_wav(&format!("{}/grains.wav", self.dir), &self.grains, 2);
        write_wav(&format!("{}/output.wav", self.dir), &self.output, 2);
        let text: String = self.markers.iter().map(|(m, f)| format!("{} {}\n", m, f)).collect();
        let _ = std::fs::write(format!("{}/markers.txt", self.dir), text);
    }
}

fn write_wav(path: &str, data: &[f32], channels: u16) {
    let mut out = Vec::with_capacity(44 + data.len() * 4);
    let data_bytes = (data.len() * 4) as u32;
    let sr = SR as u32;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sr.to_le_bytes());
    out.extend_from_slice(&(sr * channels as u32 * 4).to_le_bytes());
    out.extend_from_slice(&(channels * 4).to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for s in data {
        out.extend_from_slice(&s.to_le_bytes());
    }
    let _ = std::fs::write(path, out);
}

#[inline]
fn cell_index(x: usize, y: usize) -> usize {
    x * H + y
}

impl Node {
    fn rnd(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    fn slot_len(&self) -> usize {
        let time = self.settings.time.get().clamp(0.1, 2.0);
        ((SR * time) as usize / W).max(64)
    }

    fn grain_len(&self) -> usize {
        let size = self.settings.size.get().clamp(1.0, 4.0);
        ((self.slot_len() as f32 * size) as usize).clamp(FADE * 2, GRAIN_MAX)
    }

    /// Mark a newborn cell: it records the input when the playhead passes its
    /// column, then replays it one Time later.
    fn birth_pending(&mut self, x: usize, y: usize) {
        let ci = cell_index(x, y);
        let phase = self.rnd();
        self.cells[ci] = Cell { alive: true, pending: true, age: 0, gain: 1.0, len: 0, phase };
    }

    /// Record the last `grain_len` samples of input into cell (x, y): the slice
    /// that just played during column x (plus the smear tail before it).
    fn capture(&mut self, x: usize, y: usize) {
        let len = self.grain_len();
        let start = self.mic_write.saturating_sub(len as u64);
        let ci = cell_index(x, y);
        let base = ci * GRAIN_MAX;
        for i in 0..len {
            let p = start + i as u64;
            let s = if self.mic_write - p <= MIC_HIST as u64 && p < self.mic_write {
                self.mic_hist[(p as usize) & (MIC_HIST - 1)]
            } else {
                0.0
            };
            self.grains[base + i] = s;
        }
        let c = &mut self.cells[ci];
        c.pending = false;
        c.len = len;
        if self.debug {
            let e: f32 = self.grains[base..base + len].iter().map(|v| v * v).sum::<f32>() / len as f32;
            log!("record ({},{}) len {} rms {:.4}", x, y, len, e.sqrt());
        }
    }

    /// Slot `x` just ended: record every pending cell in that column.
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
        let r = self.settings.repeats.get().clamp(0.0, 1.0);
        match self.settings.mode.get() {
            DelayMode::Digital => r * 0.98,
            _ => {
                if r < 0.75 {
                    r * 1.3
                } else {
                    0.975 + (r - 0.75) * 0.4 // 0.975 .. 1.075
                }
            }
        }
    }

    /// Apply UI edits: `steps` differing from what we published means the
    /// user clicked / seeded / the voice stamped. Births capture input.
    fn apply_ui_edits(&mut self) {
        let mut wanted = [0u32; W];
        for x in 0..W {
            wanted[x] = self.settings.steps[x].get() & 0xFFFF;
        }
        if wanted == self.published {
            return;
        }
        if self.debug {
            log!("grid edit: {:?} -> {:?} (slot {}, mode {:?})", self.published, wanted, self.slot, self.settings.mode.get());
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
    }

    fn publish(&mut self) {
        let steps = self.grid.to_steps();
        let mut pop = 0u32;
        for x in 0..W {
            self.settings.steps[x].set(steps[x]);
            self.published[x] = steps[x];
            pop += steps[x].count_ones();
        }
        self.shared.population.store(pop, Ordering::Relaxed);
        SignalToUI::set_ui_signal();
    }

    /// One delay generation.
    fn generation(&mut self) {
        let repeats = self.regen_gain();
        if self.settings.life.get() {
            let next = self.grid.step_with_parents(&mut self.parents);
            // pass 1: births inherit from parents that are all still intact
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
            // pass 2: survivors age, the dead drop their grain
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
        // repeats that have decayed away die (Digital and low Repeats)
        for x in 0..W {
            for y in 0..H {
                let ci = cell_index(x, y);
                if self.grid.get(x, y) && self.cells[ci].gain < 0.002 {
                    self.grid.set(x, y, false);
                    self.kill(x, y);
                }
            }
        }
        self.shared.generation.fetch_add(1, Ordering::Relaxed);
        self.publish();
        if self.debug {
            let pending = self.cells.iter().filter(|c| c.alive && c.pending).count();
            log!("generation {} pop {} pending {} grid {:?}", self.shared.generation.load(Ordering::Relaxed), self.shared.population.load(Ordering::Relaxed), pending, self.grid.cells);
        }
    }

    /// Start of slot `x`: every live cell in the column replays its grain.
    fn trigger_slot(&mut self, x: usize) {
        let scale = self.settings.scale.get();
        let mode = self.settings.mode.get() as u8;
        let pitch = self.settings.pitch.get().clamp(0.0, 1.0);
        let reverse = self.settings.reverse.get().clamp(0.0, 1.0);
        let spray = self.settings.spray.get().clamp(0.0, 1.0);
        let density = self.settings.density.get().clamp(0.0, 1.0);
        let width = self.settings.width.get().clamp(0.0, 1.0);
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
            // Silos-style per-generation character
            let (cutoff, dec, noise, drive, wow_depth) = match mode {
                // Digital: clean, darkens each generation
                0 => ((8000.0 * 0.7f32.powf(age)).max(400.0), 1u8, 0.0, 1.0, 0.0),
                // Analog: darker, loses fidelity (sample-hold + hiss)
                1 => ((4500.0 * 0.85f32.powf(age)).max(400.0), (1 + (c.age as u8) / 2).min(8), 0.004 * age.min(6.0), 1.0, 0.0),
                // Tape: dark, saturates harder every cycle (more with a hot input), wow grows
                _ => (
                    (3500.0 * 0.92f32.powf(age)).max(500.0),
                    1,
                    0.0,
                    1.0 + (1.0 + 8.0 * level.min(0.5)) * age.min(10.0),
                    0.003 * (1.0 + age * 0.3).min(3.0),
                ),
            };
            let lp_a = (1.0 - (-std::f32::consts::TAU * cutoff / SR).exp()).min(1.0);
            let rng = self.seed ^ (ci as u32 * 2654435761);
            if self.debug {
                log!("trigger ({},{}) age {} gain {:.3} mode {} cutoff {:.0} lp_a {:.4} dec {} drive {:.2}", x, y, c.age, c.gain, mode, cutoff, lp_a, dec, drive);
            }
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

    /// Multi-generational degradation of one sample (coefficients fixed per voice).
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
                // xorshift hiss
                v.rng ^= v.rng << 13;
                v.rng ^= v.rng >> 17;
                v.rng ^= v.rng << 5;
                let n = (v.rng >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
                // hiss rides on the signal, never on silence
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
}

impl AudioGraphNode for Node {
    fn handle_midi_data(&mut self, data: MidiData) {
        if let Some(s) = &mut self.synth {
            s.handle_midi_data(data)
        }
    }

    fn all_notes_off(&mut self) {
        if let Some(s) = &mut self.synth {
            s.all_notes_off()
        }
    }

    fn render_to_audio_buffer(
        &mut self,
        info: AudioInfo,
        outputs: &mut [&mut AudioBuffer],
        inputs: &[&AudioBuffer],
        display: &mut DisplayAudioGraph,
    ) {
        let render_start = std::time::Instant::now();
        let output = &mut outputs[0];
        let frames = output.frame_count();
        let channels = output.channel_count();

        // 1. synth
        self.synth_buf.resize_like(output);
        self.synth_buf.zero();
        if let Some(s) = &mut self.synth {
            s.render_to_audio_buffer(info, &mut [&mut self.synth_buf], inputs, display);
        }

        // 2. pull the mic into our history (keep ~4 buffers of latency so the
        // input callback's jitter never starves us)
        let wp = MIC_RING.write_pos();
        let mut rp = match self.mic_read {
            Some(rp) if rp <= wp && wp - rp < (crate::mic_ring::MIC_RING_LEN as u64) / 2 => rp,
            _ => wp.saturating_sub(4 * frames as u64),
        };
        let mut mic = [0.0f32; 4096];
        let mut sum = 0.0f32;
        let mut underruns = 0u32;
        for i in 0..frames.min(4096) {
            let m = MIC_RING.get(rp, wp);
            if rp < wp {
                rp += 1;
            } else if wp > 0 {
                // (before the input device delivers anything there is nothing to starve)
                underruns += 1;
            }
            mic[i] = m;
            sum += m * m;
            self.mic_hist[(self.mic_write as usize) & (MIC_HIST - 1)] = m;
            self.mic_write += 1;
        }
        self.mic_read = Some(rp);
        if underruns > 0 {
            self.shared.mic_underruns.fetch_add(underruns, Ordering::Relaxed);
        }
        let rms = (sum / frames.max(1) as f32).sqrt();
        self.input_level += (rms - self.input_level) * 0.2;

        // voice presence: measured on the newest input in the ring (not the
        // 4-buffer-late read position) so the gate and the duck react fast
        let th = self.settings.gate_thresh.get().max(1e-4);
        let fresh = {
            let n = (frames as u64).min(wp).min(4096);
            let mut s = 0.0f32;
            for i in 0..n {
                let m = MIC_RING.get(wp - n + i, wp);
                s += m * m;
            }
            (s / n.max(1) as f32).sqrt()
        };
        self.present = if self.present { fresh > th * 0.5 } else { fresh > th };
        let target = if self.present { 1.0 } else { 0.0 };
        // one-pole coefficients per buffer: 5 ms attack, duck release 200 ms, gate release as set
        let attack_a = 1.0 - (-1.0 / (0.005 * SR)).exp();
        let duck_a = if self.present { attack_a } else { 1.0 - (-1.0 / (0.2 * SR)).exp() };
        let gate_rel = self.settings.gate_release.get().clamp(0.03, 2.0);
        let gate_a = if self.present { attack_a } else { 1.0 - (-1.0 / (gate_rel * SR)).exp() };
        let gate_on = self.settings.gate.get();

        // 3. UI edits to the grid
        self.apply_ui_edits();

        // 4. clock
        let running = self.settings.running.get();
        if running && !self.was_running {
            self.slot = W - 1;
            self.samples_to_slot = 0;
        }
        self.was_running = running;
        let freeze = self.settings.freeze.get();

        // mix: equal-power dry voice / wet repeats
        let mix = self.settings.mix.get().clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
        let dry = mix.cos();
        let wet = mix.sin();
        let duck = self.settings.duck.get().clamp(0.0, 1.0);
        let sg = self.settings.synth_gain.get();
        let dt = 1.0 / SR;
        // return filter: tone < 0 closes a low-pass (16 kHz .. 500 Hz),
        // tone > 0 raises a high-pass (30 Hz .. 2 kHz); sub-30 Hz rumble always cut
        let tone = self.settings.tone.get().clamp(-1.0, 1.0);
        let lp_cut = 16000.0 * (500.0f32 / 16000.0).powf((-tone).max(0.0));
        let hp_cut = 30.0 * (2000.0f32 / 30.0).powf(tone.max(0.0));
        let lp_a = 1.0 - (-std::f32::consts::TAU * lp_cut / SR).exp();
        let hp_a = 1.0 - (-std::f32::consts::TAU * hp_cut / SR).exp();

        let mut disp = display.pop_buffer_resize(frames, channels);
        if let Some(d) = &mut disp {
            d.zero();
        }
        if let Some(rec) = &mut self.rec {
            let m = self.shared.marker.load(Ordering::Relaxed);
            if m != rec.last_marker {
                rec.last_marker = m;
                rec.markers.push((m, rec.input.len()));
            }
        }

        let mut peak = 0.0f32;
        for i in 0..frames {
            if running {
                if self.samples_to_slot == 0 {
                    // frozen: the grid and the grains hold, the playhead keeps replaying
                    if !freeze {
                        self.record_pending(self.slot);
                    }
                    self.slot = (self.slot + 1) % W;
                    if self.slot == 0 && !freeze {
                        self.generation();
                    }
                    self.samples_to_slot = self.slot_len() as u32;
                    self.trigger_slot(self.slot);
                    self.shared.current_slot.store(self.slot as u32, Ordering::Relaxed);
                }
                self.samples_to_slot -= 1;
            }

            // presence envelopes
            self.duck_env += (target - self.duck_env) * duck_a;
            self.gate_env += (target - self.gate_env) * gate_a;

            // voices
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
                // reverse reads the grain from its end; the window stays symmetric
                let rp = if v.reverse { (v.len - 1) as f64 - v.pos } else { v.pos };
                let ip = rp.floor();
                let f = (rp - ip) as f32;
                let i0 = (ip as usize).min(v.len - 2);
                let s = self.grains[base + i0] * (1.0 - f) + self.grains[base + i0 + 1] * f;
                // 5 ms edges
                let edge_in = (pi as f32 / FADE as f32).min(1.0);
                let edge_out = ((v.len - pi) as f32 / FADE as f32).min(1.0);
                let win = edge_in * edge_out;
                let s = Self::degrade(v, s) * win * v.gain;
                gl += s * v.gain_l;
                gr += s * v.gain_r;
                // tape wow: slow pitch wobble (cheap parabolic sine)
                let wow = if v.wow_depth > 0.0 {
                    v.wow_phase = (v.wow_phase + dt * 0.9).fract();
                    let ph = v.wow_phase * 2.0 - 1.0;
                    1.0 + v.wow_depth * (ph * (1.0 - ph.abs()) * 4.0)
                } else {
                    1.0
                };
                v.pos += v.rate * wow as f64;
            }

            // return: tone filter, duck, wet level
            let ret_gain = wet * (1.0 - duck * self.duck_env);
            let mut ret = [gl, gr];
            for c in 0..2 {
                self.tone_lp[c] += (ret[c] - self.tone_lp[c]) * lp_a;
                let x = self.tone_lp[c];
                self.tone_hp[c] += (x - self.tone_hp[c]) * hp_a;
                ret[c] = ((x - self.tone_hp[c]) * ret_gain).clamp(-1.5, 1.5);
            }
            let (gl, gr) = (ret[0], ret[1]);
            peak = peak.max(gl.abs()).max(gr.abs());
            let m = mic[i.min(4095)];
            let synth_g = if gate_on { sg * self.gate_env } else { sg };
            for c in 0..channels {
                let synth = self.synth_buf.channel(c)[i] * synth_g;
                let grain = if c == 0 { gl } else { gr };
                output.channel_mut(c)[i] = synth + m * dry + grain;
                if let Some(d) = &mut disp {
                    d.channel_mut(c)[i] = grain;
                }
            }
            if let Some(rec) = &mut self.rec {
                if !rec.done && !rec.full() {
                    rec.input.push(m);
                    rec.grains.push(gl);
                    rec.grains.push(gr);
                    rec.output.push(output.channel(0)[i]);
                    rec.output.push(output.channel(channels.min(2) - 1)[i]);
                }
            }
        }
        self.shared.grain_level.fetch_max(peak.to_bits(), Ordering::Relaxed);
        let us = render_start.elapsed().as_micros().min(u32::MAX as u128) as u32;
        self.shared.max_render_us.fetch_max(us, Ordering::Relaxed);
        self.shared.frames.store(frames as u32, Ordering::Relaxed);
        // QA only: the one-time file write is deliberately outside the timing
        if let Some(rec) = &mut self.rec {
            if !rec.done && (rec.full() || self.shared.record_stop.load(Ordering::Relaxed) == 1) {
                rec.write();
                self.shared.record_stop.store(2, Ordering::Relaxed);
            }
        }
        if let Some(d) = disp {
            display.send_buffer(peak > 0.0005, 16, d);
        }
    }
}

impl GranularDelay {
    pub fn current_slot(&self) -> usize {
        self.shared.current_slot.load(Ordering::Relaxed) as usize
    }

    pub fn generation(&self) -> u32 {
        self.shared.generation.load(Ordering::Relaxed)
    }

    /// Peak repeat level since the last call.
    pub fn take_grain_level(&self) -> f32 {
        f32::from_bits(self.shared.grain_level.swap(0, Ordering::Relaxed))
    }
}
