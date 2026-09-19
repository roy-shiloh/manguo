//! Grain sequencer: a granular delay of the microphone, driven by the 16x16
//! Life grid, mixed with the synth.
//!
//! The component is the audio-graph root and owns the synth as a child. On
//! the audio thread it pulls mic samples from `MIC_RING`, writes them into a
//! delay line, and on every sequencer step spawns one grain per live cell in
//! that step's column, reading back `delay_steps` steps into the past and
//! pitch-shifted by the cell's row (quantized to the selected scale).

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
const DELAY_LEN: usize = 1 << 19; // ~10.9 s at 48 kHz
const MAX_GRAINS: usize = 64;
const GRID_W: usize = 16;
const GRID_H: usize = 16;

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

    /// Scale degree (may be negative) to semitones.
    fn semitones(&self, degree: i32) -> i32 {
        let iv = self.intervals();
        let n = iv.len() as i32;
        let oct = degree.div_euclid(n);
        let idx = degree.rem_euclid(n) as usize;
        oct * 12 + iv[idx]
    }
}

#[derive(Live, LiveHook, LiveRegister, LiveAtomic, Debug, LiveRead)]
#[live_ignore]
pub struct GranularSettings {
    /// the Life grid: column = step, bit = row (row 0 is the top / highest pitch)
    #[live]
    pub steps: [u32a; 16],
    #[live(125.0)]
    pub bpm: f32a,
    #[live(false)]
    pub running: boola,
    #[live]
    pub scale: U32A<GrainScale>,
    /// how far back a grain reads, in sequencer steps
    #[live(4.0)]
    pub delay_steps: f32a,
    /// grain length 0..1 -> 30 ms .. 600 ms
    #[live(0.45)]
    pub grain_len: f32a,
    /// random offset of the read position, 0..1 -> 0 .. 2 steps
    #[live(0.2)]
    pub spray: f32a,
    /// grains written back into the delay line
    #[live(0.3)]
    pub feedback: f32a,
    /// grain output level
    #[live(0.8)]
    pub wet: f32a,
    /// dry microphone level (0 by default: speakers would feed back)
    #[live(0.0)]
    pub voice_gain: f32a,
    #[live(1.0)]
    pub synth_gain: f32a,
}

/// Published by the audio thread for the UI.
#[derive(Default)]
pub struct GranularShared {
    pub current_step: AtomicU32,
    /// peak grain level of the last buffer, f32 bits
    pub grain_level: AtomicU32,
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
        // the single anonymous child is the synth
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
            delay: vec![0.0; DELAY_LEN],
            write: 0,
            mic_read: None,
            grains: [Grain::default(); MAX_GRAINS],
            samples_to_step: 0,
            step: GRID_W - 1,
            was_running: false,
            seed: 0x9E37_79B9,
            fb_l: 0.0,
            fb_r: 0.0,
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

#[derive(Clone, Copy, Default)]
struct Grain {
    active: bool,
    /// absolute read position in delay-line frames (fractional)
    pos: f64,
    rate: f64,
    len: u32,
    age: u32,
    gain_l: f32,
    gain_r: f32,
}

struct Node {
    settings: Arc<GranularSettings>,
    shared: Arc<GranularShared>,
    synth: Option<Box<dyn AudioGraphNode + Send>>,
    synth_buf: AudioBuffer,
    delay: Vec<f32>,
    /// absolute frames written to the delay line
    write: u64,
    /// absolute mic-ring frame we read next
    mic_read: Option<u64>,
    grains: [Grain; MAX_GRAINS],
    samples_to_step: u32,
    step: usize,
    was_running: bool,
    seed: u32,
    fb_l: f32,
    fb_r: f32,
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

    fn samples_per_step(&self) -> f32 {
        let bpm = self.settings.bpm.get().clamp(20.0, 300.0);
        SR * 60.0 / bpm / 4.0
    }

    fn spawn_grains(&mut self) {
        let column = self.settings.steps[self.step].get();
        if column == 0 {
            return;
        }
        let sps = self.samples_per_step() as f64;
        let scale = self.settings.scale.get();
        let len = (SR * (0.03 + 0.57 * self.settings.grain_len.get().clamp(0.0, 1.0))) as u32;
        let spray = self.settings.spray.get().clamp(0.0, 1.0) as f64 * 2.0 * sps;
        let back = self.settings.delay_steps.get().clamp(1.0, 16.0) as f64 * sps;
        for row in 0..GRID_H {
            if column & (1 << row) == 0 {
                continue;
            }
            // row 0 = top = highest degree
            let degree = 8 - row as i32;
            let semis = scale.semitones(degree);
            let rate = 2f64.powf(semis as f64 / 12.0);
            let jitter = (self.rnd() as f64 * 2.0 - 1.0) * spray;
            let start = (self.write as f64 - back - jitter - len as f64 * rate).max(0.0);
            let pan = (row as f32 / (GRID_H - 1) as f32) * 1.4 - 0.7; // -0.7..0.7
            let gl = ((1.0 - pan) * 0.5).sqrt();
            let gr = ((1.0 + pan) * 0.5).sqrt();
            if let Some(g) = self.grains.iter_mut().find(|g| !g.active) {
                *g = Grain { active: true, pos: start, rate, len, age: 0, gain_l: gl, gain_r: gr };
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
        let output = &mut outputs[0];
        let frames = output.frame_count();
        let channels = output.channel_count();

        // 1. synth
        self.synth_buf.resize_like(output);
        self.synth_buf.zero();
        if let Some(s) = &mut self.synth {
            s.render_to_audio_buffer(info, &mut [&mut self.synth_buf], inputs, display);
        }

        // 2. mic position bookkeeping (keep ~2 buffers of latency)
        let wp = MIC_RING.write_pos();
        let mut rp = match self.mic_read {
            Some(rp) if rp <= wp && wp - rp < (crate::mic_ring::MIC_RING_LEN as u64) / 2 => rp,
            _ => wp.saturating_sub(2 * frames as u64),
        };

        // 3. sequencer clock state
        let running = self.settings.running.get();
        if running && !self.was_running {
            self.step = GRID_W - 1;
            self.samples_to_step = 0;
        }
        self.was_running = running;

        let wet = self.settings.wet.get();
        let dry = self.settings.voice_gain.get();
        let sg = self.settings.synth_gain.get();
        let fb = self.settings.feedback.get().clamp(0.0, 0.95);

        // grains also go to the oscilloscope, as display voice 16
        let mut disp = display.pop_buffer_resize(frames, channels);
        if let Some(d) = &mut disp {
            d.zero();
        }

        let mut peak = 0.0f32;
        for i in 0..frames {
            // step clock
            if running {
                if self.samples_to_step == 0 {
                    self.step = (self.step + 1) % GRID_W;
                    self.samples_to_step = self.samples_per_step().max(1.0) as u32;
                    self.spawn_grains();
                    self.shared.current_step.store(self.step as u32, Ordering::Relaxed);
                    SignalToUI::set_ui_signal();
                }
                self.samples_to_step -= 1;
            }

            // grains
            let mut gl = 0.0f32;
            let mut gr = 0.0f32;
            for g in self.grains.iter_mut() {
                if !g.active {
                    continue;
                }
                let s = {
                    // inline of delay_read (borrow rules)
                    let pos = g.pos;
                    if pos < 0.0 || self.write as f64 - pos > (DELAY_LEN - 2) as f64 || pos >= self.write as f64 {
                        0.0
                    } else {
                        let fl = pos.floor();
                        let f = (pos - fl) as f32;
                        let i0 = (fl as usize) & (DELAY_LEN - 1);
                        let i1 = (i0 + 1) & (DELAY_LEN - 1);
                        self.delay[i0] * (1.0 - f) + self.delay[i1] * f
                    }
                };
                // Hann window
                let t = g.age as f32 / g.len.max(1) as f32;
                let w = 0.5 - 0.5 * (t * std::f32::consts::TAU).cos();
                gl += s * w * g.gain_l;
                gr += s * w * g.gain_r;
                g.pos += g.rate;
                g.age += 1;
                if g.age >= g.len {
                    g.active = false;
                }
            }

            // mic in -> delay line (+ feedback of the previous frame's grains)
            let m = MIC_RING.get(rp, wp);
            if rp < wp {
                rp += 1;
            }
            let fbin = (self.fb_l + self.fb_r) * 0.5 * fb;
            self.delay[(self.write as usize) & (DELAY_LEN - 1)] = (m + fbin).clamp(-1.5, 1.5);
            self.write += 1;
            self.fb_l = gl;
            self.fb_r = gr;

            let gl = gl * wet;
            let gr = gr * wet;
            peak = peak.max(gl.abs()).max(gr.abs());

            for c in 0..channels {
                let synth = self.synth_buf.channel(c)[i] * sg;
                let grain = if c == 0 { gl } else { gr };
                output.channel_mut(c)[i] = synth + m * dry + grain;
                if let Some(d) = &mut disp {
                    d.channel_mut(c)[i] = grain;
                }
            }
        }
        self.mic_read = Some(rp);
        self.shared.grain_level.store(peak.to_bits(), Ordering::Relaxed);
        if let Some(d) = disp {
            display.send_buffer(peak > 0.0005, 16, d);
        }
    }
}

impl GranularDelay {
    pub fn current_step(&self) -> usize {
        self.shared.current_step.load(Ordering::Relaxed) as usize
    }
}
