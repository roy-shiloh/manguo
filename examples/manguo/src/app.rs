use crate::makepad_platform::live_atomic::AtomicGetSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use crate::{
    granular::*, life::*, mandelbrot::*, makepad_audio_graph::*, mic_ring::MIC_RING, makepad_audio_widgets::display_audio::*,
    makepad_audio_widgets::piano::*, makepad_synth_ironfish::ironfish::*, makepad_widgets::*,
    sequencer::*,
};

live_design! {
    use link::widgets::*
    use link::theme::*
    use manguo::app_desktop::AppDesktop
    use manguo::app_desktop::LifeWindow

    use makepad_audio_graph::instrument::Instrument;
    use makepad_synth_ironfish::ironfish::IronFish;
    use manguo::granular::GranularDelay;

    App = {{App}} {

        audio_graph: {
            root: <GranularDelay> {
                synth = <Instrument> {
                    <IronFish> {}
                }
            }
        }
        ui: <Root>{
            main_window = <Window> {
                window: {inner_size: vec2(1400, 870), title: "MANGUO 芒果"},
                pass: {clear_color: #x1A0A14}
                block_signal_event: true;
                body = <AppDesktop> {}
            }
            life_window = <Window> {
                window: {inner_size: vec2(720, 800), title: "MANGUO LIFE", open_on_start: false},
                pass: {clear_color: #x1A0A14}
                block_signal_event: true;
                body = <LifeWindow> {}
            }
        }
    }
}
app_main!(App);

/// Published by the audio-input callback, read on the UI thread. Lock-free.
pub struct VoiceShared {
    /// RMS level of the last input buffer, as f32 bits
    level: AtomicU32,
    /// Estimated fundamental in Hz (0 when unsure), as f32 bits
    pitch_hz: AtomicU32,
    /// Mix > Input: mic gain applied before detection, grains and dry path, f32 bits
    input_gain: AtomicU32,
}

impl Default for VoiceShared {
    fn default() -> Self {
        Self { level: AtomicU32::new(0), pitch_hz: AtomicU32::new(0), input_gain: AtomicU32::new(1.0f32.to_bits()) }
    }
}

impl VoiceShared {
    fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }
    fn pitch(&self) -> f32 {
        f32::from_bits(self.pitch_hz.load(Ordering::Relaxed))
    }
}

/// Voice-to-synth control state (UI thread).
pub struct VoiceControl {
    pub enabled: bool,
    /// RMS threshold for a note-on
    pub threshold: f32,
    /// Randomization amount 0..1
    pub chaos: f32,
    pub active_note: Option<u8>,
    pub base_note: u8,
    pub interval: i8,
    pub last_rand_time: f64,
    pub silence_since: Option<f64>,
    pub seed: u32,
    pub shared: Arc<VoiceShared>,
    pub smoothed_level: f32,
}

impl Default for VoiceControl {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 0.03,
            chaos: 0.35,
            active_note: None,
            base_note: 0,
            interval: 0,
            last_rand_time: 0.0,
            silence_since: None,
            seed: 0x5EED_1234,
            shared: Arc::new(VoiceShared::default()),
            smoothed_level: 0.0,
        }
    }
}

/// UI-side state for the Life grain delay (the grid itself lives on the audio thread).
#[derive(Default)]
pub struct LifeControl {
    pub enabled: bool,
    pub seed: u32,
    /// last generation counter seen from the audio thread
    pub last_generation: u32,
    /// dev: one-shot timer that closes the life window (MANGUO_CLOSE_LIFE_AFTER)
    pub test_close_timer: Timer,
    /// last playhead column shown
    pub last_step: usize,
    /// dev: MANGUO_LOG_GRAINS logs grain peaks once a second
    pub log_grains: bool,
    pub last_grain_log: f64,
    /// dev: MANGUO_QA / MANGUO_AUDIO_QA scripted runs
    pub qa_timer: Timer,
    pub qa_step: usize,
    pub audio_qa: bool,
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    audio_graph: AudioGraph,
    #[rust]
    midi_input: MidiInput,
    #[rust(LifeControl { seed: 0xC0FFEE, ..Default::default() })]
    life: LifeControl,
    #[rust]
    voice: VoiceControl,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        crate::makepad_audio_widgets::live_design(cx);
        crate::makepad_audio_graph::live_design(cx);
        crate::makepad_synth_ironfish::live_design(cx);
        crate::granular::live_design(cx);
        crate::sequencer::live_design(cx);
        crate::mandelbrot::live_design(cx);
        crate::app_desktop::live_design(cx);
    }
}

impl LiveHook for App{
    fn after_apply(&mut self, cx: &mut Cx, apply: &mut Apply, _index: usize, _nodes: &[LiveNode]) {
        if apply.from.is_update_from_doc(){
            self.init_ui_state(cx);
        }
    }
}

impl App {
    pub fn init_ui_state(&mut self, cx:&mut Cx){
        let ui = self.ui.clone();
        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        let db = DataBindingStore::from_nodes(ironfish.settings.live_read());
        Self::data_bind(db.data_to_widgets(cx, &ui));
        let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
        let db = DataBindingStore::from_nodes(granular.settings.live_read());
        Self::granular_bind(db.data_to_widgets(cx, &ui));
    }
    
    /// Bindings for the grain sequencer (GranularSettings).
    pub fn granular_bind(mut db: DataBindingMap) {
        db.bind(id!(steps), ids!(sequencer));
        db.bind(id!(running), ids!(playpause.checkbox));
        db.bind(id!(freeze), ids!(freeze.checkbox));
        db.bind(id!(time), ids!(delaytime.slider));
        db.bind(id!(repeats), ids!(repeats.slider));
        db.bind(id!(mode), ids!(delaymode.dropdown));
        db.bind(id!(scale), ids!(scaletype.dropdown));
        db.bind(id!(pitch), ids!(grainpitch.slider));
        db.bind(id!(size), ids!(grainsize.slider));
        db.bind(id!(reverse), ids!(grainreverse.slider));
        db.bind(id!(spray), ids!(grainspray.slider));
        db.bind(id!(density), ids!(graindensity.slider));
        db.bind(id!(width), ids!(grainwidth.slider));
        db.bind(id!(tone), ids!(returntone.slider));
        db.bind(id!(duck), ids!(returnduck.slider));
        db.bind(id!(mix), ids!(mixdrywet.slider));
        db.bind(id!(synth_gain), ids!(synthlevel.slider));
        db.bind(id!(gate), ids!(synthgate.checkbox));
        db.bind(id!(gate_release), ids!(gaterelease.slider));
    }

    /// Bindings for the synth (IronFishSettings).
    pub fn data_bind(mut db: DataBindingMap) {
        db.bind(id!(arp.enabled), ids!(arp.checkbox));
        db.bind(id!(arp.octaves), ids!(arpoctaves.slider));
        db.bind(id!(sequencer.bpm), ids!(arprate.slider));

        // Mixer panel
        db.bind(id!(osc_balance), ids!(balance.slider));
        db.bind(id!(noise), ids!(noise.slider));
        db.bind(id!(sub_osc), ids!(sub.slider));
        db.bind(id!(portamento), ids!(porta.slider));

        // DelayFX Panel
        db.bind(id!(delay.delaysend), ids!(delaysend.slider));
        db.bind(id!(delay.delayfeedback), ids!(delayfeedback.slider));

        db.bind(id!(bitcrush.enable), ids!(crushenable.checkbox));
        db.bind(id!(bitcrush.amount), ids!(crushamount.slider));

        db.bind(id!(delay.difference), ids!(delaydifference.slider));
        db.bind(id!(delay.cross), ids!(delaycross.slider));
        db.bind(id!(delay.length), ids!(delaylength.slider));

        // Chorus panel
        db.bind(id!(chorus.mix), ids!(chorusmix.slider));
        db.bind(id!(chorus.mindelay), ids!(chorusdelay.slider));
        db.bind(id!(chorus.moddepth), ids!(chorusmod.slider));
        db.bind(id!(chorus.rate), ids!(chorusrate.slider));
        db.bind(id!(chorus.phasediff), ids!(chorusphase.slider));
        db.bind(id!(chorus.feedback), ids!(chorusfeedback.slider));

        // Reverb panel
        db.bind(id!(reverb.mix), ids!(reverbmix.slider));
        db.bind(id!(reverb.feedback), ids!(reverbfeedback.slider));

        //LFO Panel
        db.bind(id!(lfo.rate), ids!(rate.slider));
        db.bind(id!(filter1.lfo_amount), ids!(lfoamount.slider));
        db.bind(id!(lfo.synconkey), ids!(sync.checkbox));
        
        //Volume Envelope
        db.bind(id!(volume_envelope.a), ids!(vol_env.attack.slider));
        db.bind(id!(volume_envelope.h), ids!(vol_env.hold.slider));
        db.bind(id!(volume_envelope.d), ids!(vol_env.decay.slider));
        db.bind(id!(volume_envelope.s), ids!(vol_env.sustain.slider));
        db.bind(id!(volume_envelope.r), ids!(vol_env.release.slider));

        //Mod Envelope
        db.bind(id!(mod_envelope.a), ids!(mod_env.attack.slider));
        db.bind(id!(mod_envelope.h), ids!(mod_env.hold.slider));
        db.bind(id!(mod_envelope.d), ids!(mod_env.decay.slider));
        db.bind(id!(mod_envelope.s), ids!(mod_env.sustain.slider));
        db.bind(id!(mod_envelope.r), ids!(mod_env.release.slider));
        db.bind(id!(filter1.envelope_amount), ids!(modamount.slider));

        // Filter panel
        db.bind(id!(filter1.filter_type), ids!(filter_type));
        db.bind(id!(filter1.cutoff), ids!(cutoff.slider));
        db.bind(id!(filter1.resonance), ids!(resonance.slider));

        // Osc1 panel
        db.bind(id!(supersaw1.spread), ids!(osc1.supersaw.spread.slider));
        db.bind(id!(supersaw1.diffuse), ids!(osc1.supersaw.diffuse.slider));
        db.bind(id!(supersaw1.spread), ids!(osc1.hypersaw.spread.slider));
        db.bind(id!(supersaw1.diffuse), ids!(osc1.hypersaw.diffuse.slider));

        db.bind(id!(osc1.osc_type), ids!(osc1.type.dropdown));
        db.bind(id!(osc1.transpose), ids!(osc1.transpose.slider));
        db.bind(id!(osc1.detune), ids!(osc1.detune.slider));
        db.bind(id!(osc1.harmonic), ids!(osc1.harmonicshift.slider));
        db.bind(id!(osc1.harmonicenv), ids!(osc1.harmonicenv.slider));
        db.bind(id!(osc1.harmoniclfo), ids!(osc1.harmoniclfo.slider));

        // Osc2 panel
        db.bind(id!(supersaw2.spread), ids!(osc2.supersaw.spread.slider));
        db.bind(id!(supersaw2.diffuse), ids!(osc2.supersaw.diffuse.slider));
        db.bind(id!(supersaw2.spread), ids!(osc2.hypersaw.spread.slider));
        db.bind(id!(supersaw2.diffuse), ids!(osc2.hypersaw.diffuse.slider));

        db.bind(id!(osc2.osc_type), ids!(osc2.type.dropdown));
        db.bind(id!(osc2.transpose), ids!(osc2.transpose.slider));
        db.bind(id!(osc2.detune), ids!(osc2.detune.slider));
        db.bind(id!(osc2.harmonic), ids!(osc2.harmonicshift.slider));
        db.bind(id!(osc2.harmonicenv), ids!(osc2.harmonicenv.slider));
        db.bind(id!(osc2.harmoniclfo), ids!(osc2.harmoniclfo.slider));

        db.apply(id!(osc1.osc_type), ids!(osc1.supersaw, visible), |v| {
            v.enum_eq(id!(SuperSaw))
        });
        db.apply(id!(osc2.osc_type), ids!(osc2.supersaw, visible), |v| {
            v.enum_eq(id!(SuperSaw))
        });
        db.apply(id!(osc1.osc_type), ids!(osc1.hypersaw, visible), |v| {
            v.enum_eq(id!(HyperSaw))
        });
        db.apply(id!(osc2.osc_type), ids!(osc2.hypersaw, visible), |v| {
            v.enum_eq(id!(HyperSaw))
        });
        db.apply(id!(osc1.osc_type), ids!(osc1.harmonic, visible), |v| {
            v.enum_eq(id!(HarmonicSeries))
        });
        db.apply(id!(osc2.osc_type), ids!(osc2.harmonic, visible), |v| {
            v.enum_eq(id!(HarmonicSeries))
        });

        db.apply(
            id!(mod_envelope.a),
            ids!(mod_env.display, draw_bg.attack),
            |v| v,
        );
        db.apply(
            id!(mod_envelope.h),
            ids!(mod_env.display, draw_bg.hold),
            |v| v,
        );
        db.apply(
            id!(mod_envelope.d),
            ids!(mod_env.display, draw_bg.decay),
            |v| v,
        );
        db.apply(
            id!(mod_envelope.s),
            ids!(mod_env.display, draw_bg.sustain),
            |v| v,
        );
        db.apply(
            id!(mod_envelope.r),
            ids!(mod_env.display, draw_bg.release),
            |v| v,
        );
        db.apply(
            id!(volume_envelope.a),
            ids!(vol_env.display, draw_bg.attack),
            |v| v,
        );
        db.apply(
            id!(volume_envelope.h),
            ids!(vol_env.display, draw_bg.hold),
            |v| v,
        );
        db.apply(
            id!(volume_envelope.d),
            ids!(vol_env.display, draw_bg.decay),
            |v| v,
        );
        db.apply(
            id!(volume_envelope.s),
            ids!(vol_env.display, draw_bg.sustain),
            |v| v,
        );
        db.apply(
            id!(volume_envelope.r),
            ids!(vol_env.display, draw_bg.release),
            |v| v,
        );
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        self.preset(cx,0,false);
        self.ui.piano(id!(piano)).set_key_focus(cx);
        self.midi_input = cx.midi_input();
        self.start_voice_input(cx);
        self.set_voice_threshold(self.voice.threshold);
        // Dev/verification: MANGUO_OPEN_LIFE=grid|fractal opens the life window running.
        if std::env::var("MANGUO_VOICE_TEST").is_ok() && std::env::var("MANGUO_AUDIO_QA").is_err() {
            self.set_voice_enabled(cx, true);
        }
        self.life.log_grains = std::env::var("MANGUO_LOG_GRAINS").is_ok();
        if std::env::var("MANGUO_QA").is_ok() {
            log!("QA: start");
            self.qa_schedule(cx, 1.0);
        }
        if std::env::var("MANGUO_AUDIO_QA").is_ok() {
            log!("AQA: start");
            self.life.audio_qa = true;
            self.qa_schedule(cx, 0.5);
        }
        if let Ok(mode) = std::env::var("MANGUO_OPEN_LIFE") {
            self.ui.window(id!(life_window)).open(cx);
            self.set_life_enabled(cx, true);
            self.set_life_mode(cx, mode == "fractal");
        }
        if let Some(secs) = std::env::var("MANGUO_CLOSE_LIFE_AFTER").ok().and_then(|v| v.parse::<f64>().ok()) {
            self.life.test_close_timer = cx.start_timeout(secs);
        }
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        
        let ui = self.ui.clone();
        let piano = ui.piano(id!(piano));

        for note in piano.notes_played(&actions) {
            self.audio_graph.send_midi_data(
                MidiNote {
                    channel: 0,
                    is_on: note.is_on,
                    note_number: note.note_number,
                    velocity: note.velocity,
                }
                .into(),
            );
        }

        let sequencer = ui.sequencer(id!(sequencer));

        if ui.button_set(ids!(clear_grid)).clicked(&actions) {
            sequencer.clear_grid(cx);
        }

        if ui.button_set(ids!(grid_down)).clicked(&actions) {
            sequencer.grid_down(cx);
        }

        if ui.button_set(ids!(grid_up)).clicked(&actions) {
            sequencer.grid_up(cx);
        }

        if let Some((index,km)) = ui.button_set(ids!(preset_1, preset_2, preset_3, preset_4, preset_5, preset_6, preset_7,preset_8)).which_clicked_modifiers(&actions){
            self.preset(cx, index, km.shift);
        }

        // Voice controls
        if let Some(on) = ui.check_box(id!(voice.checkbox)).changed(&actions) {
            self.set_voice_enabled(cx, on);
        }
        if let Some(v) = ui.slider(id!(voicesens.slider)).slided(&actions) {
            // 0..1 -> RMS threshold 0.15 .. 0.005 (log-ish); higher = more sensitive
            self.set_voice_threshold(0.15 * (0.005f32 / 0.15).powf(v as f32));
        }
        if let Some(v) = ui.slider(id!(voicechaos.slider)).slided(&actions) {
            self.voice.chaos = v as f32;
        }
        if let Some(v) = ui.slider(id!(mixinput.slider)).slided(&actions) {
            self.voice.shared.input_gain.store((v as f32).to_bits(), Ordering::Relaxed);
        }

        // Game of Life controls (present in both windows; ids are shared)
        let life_toggled = ui.check_box_set(ids!(life.checkbox)).changed(&actions);
        if let Some(on) = life_toggled {
            self.set_life_enabled(cx, on);
        }
        if ui.button_set(ids!(lifeseed.button)).clicked(&actions) {
            self.life_seed(cx);
        }
        if ui.button_set(ids!(lifewindow.button)).clicked(&actions) {
            ui.window(id!(life_window)).open(cx);
        }
        if ui.button(id!(tab_grid)).clicked(&actions) {
            self.set_life_mode(cx, false);
        }
        if ui.button(id!(tab_fractal)).clicked(&actions) {
            self.set_life_mode(cx, true);
        }
        if let Some(v) = ui.slider(id!(zoomspeed.slider)).slided(&actions) {
            ui.mandelbrot(id!(fractal)).set_speed(v as f32);
        }

        let mut db = DataBindingStore::new();
        db.data_bind(cx, actions, &ui, Self::data_bind);
        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        ironfish.settings.apply_over(cx, &db.nodes);

        let mut gdb = DataBindingStore::new();
        gdb.data_bind(cx, actions, &ui, Self::granular_bind);
        let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
        granular.settings.apply_over(cx, &gdb.nodes);
    }

    fn handle_midi_ports(&mut self, cx: &mut Cx, ports: &MidiPortsEvent) {
        cx.use_midi_inputs(&ports.all_inputs());
    }

    fn handle_audio_devices(&mut self, cx: &mut Cx, devices: &AudioDevicesEvent) {
        cx.use_audio_outputs(&devices.default_output());
        cx.use_audio_inputs(&devices.default_input());
    }

    fn handle_signal(&mut self, cx: &mut Cx) {
        let piano = self.ui.piano_set(ids!(piano));
        while let Some((_, data)) = self.midi_input.receive() {
            self.audio_graph.send_midi_data(data);
            if let Some(note) = data.decode().on_note() {
                piano.set_note(cx, note.is_on, note.note_number)
            }
        }
        self.voice_update(cx);
        // grain sequencer playhead
        let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
        if self.life.log_grains {
            let now = cx.seconds_since_app_start();
            if now - self.life.last_grain_log > 1.0 {
                self.life.last_grain_log = now;
                let lvl = granular.take_grain_level();
                if lvl > 0.001 {
                    log!("grains: slot {} gen {} peak {:.3}", granular.current_slot(), granular.generation(), lvl);
                }
            }
        }
        let step = granular.current_slot();
        if step != self.life.last_step {
            self.life.last_step = step;
            self.ui.sequencer_set(ids!(sequencer)).set_step(cx, step);
        }
        // a generation ran on the audio thread: mirror the grid, shift the fractal palette
        let gen = granular.generation();
        if gen != self.life.last_generation {
            self.life.last_generation = gen;
            let steps: Vec<u32> = (0..16).map(|i| granular.settings.steps[i].get()).collect();
            self.ui.sequencer_set(ids!(sequencer)).set_steps(cx, &steps);
            self.ui.mandelbrot(id!(fractal)).on_generation();
            if self.life.enabled && steps.iter().all(|s| *s == 0) && !self.life.audio_qa {
                self.life_seed(cx);
            }
        }
    }
}

// Voice control
impl App {
    /// Registers the microphone callback. It owns its analysis state, never
    /// allocates on the hot path and publishes through atomics only.
    fn start_voice_input(&mut self, cx: &mut Cx) {
        let shared = self.voice.shared.clone();
        const N: usize = 1024;
        let mut ring = [0.0f32; N];
        let mut write = 0usize;
        let mut scratch = [0.0f32; N];
        // Dev/verification: MANGUO_VOICE_TEST=<hz>[:<on_secs>[:<period_secs>]]
        // replaces the mic with a synthetic "sung" tone: on for `on_secs`,
        // repeating every `period_secs` (default: once), so the whole chain
        // can be exercised headless.
        let (test_hz, test_secs, test_period): (Option<f32>, f32, f32) = match std::env::var("MANGUO_VOICE_TEST") {
            Ok(v) => {
                let mut it = v.split(':');
                (
                    it.next().and_then(|h| h.parse().ok()),
                    it.next().and_then(|s| s.parse().ok()).unwrap_or(f32::INFINITY),
                    it.next().and_then(|s| s.parse().ok()).unwrap_or(f32::INFINITY),
                )
            }
            Err(_) => (None, 0.0, 0.0),
        };
        let mut phase = 0.0f32;
        let mut test_frames = 0u64;
        let mut chunk = [0.0f32; 4096];
        cx.audio_input(0, move |_info, input| {
            let frames = input.frame_count();
            if frames == 0 || input.channel_count() == 0 {
                return;
            }
            let ch = input.channel(0);
            let gain = f32::from_bits(shared.input_gain.load(Ordering::Relaxed));
            let mut sum = 0.0f32;
            let mut n = 0usize;
            for i in 0..frames {
                let mut s = ch[i] * gain;
                if let Some(hz) = test_hz {
                    test_frames += 1;
                    phase = (phase + hz / 48000.0).fract();
                    let t = test_frames as f32 / 48000.0;
                    let in_burst = if test_period.is_finite() { t % test_period < test_secs } else { t < test_secs };
                    s = if in_burst {
                        // band-limited saw: 12 harmonics, so darkening is measurable
                        let mut acc = 0.0f32;
                        for h in 1..=12u32 {
                            acc += (phase * h as f32 * std::f32::consts::TAU).sin() / h as f32;
                        }
                        acc * 0.16
                    } else { 0.0 };
                }
                sum += s * s;
                ring[write] = s;
                write = (write + 1) % N;
                if n < chunk.len() {
                    chunk[n] = s;
                    n += 1;
                } else {
                    MIC_RING.push(&chunk);
                    n = 0;
                    chunk[0] = s;
                    n += 1;
                }
            }
            MIC_RING.push(&chunk[..n]);
            let rms = (sum / frames as f32).sqrt();
            shared.level.store(rms.to_bits(), Ordering::Relaxed);

            // pitch via normalized autocorrelation over the ring, lags 80..1000 Hz
            let mut pitch = 0.0f32;
            if rms > 0.005 {
                for i in 0..N {
                    scratch[i] = ring[(write + i) % N];
                }
                let sr = 48000.0f32; // makepad opens inputs at 48 kHz
                let min_lag = (sr / 1000.0) as usize;
                let max_lag = ((sr / 80.0) as usize).min(N / 2);
                let mut best_lag = 0usize;
                let mut best = 0.0f32;
                let mut e0 = 0.0f32;
                for i in 0..(N - max_lag) {
                    e0 += scratch[i] * scratch[i];
                }
                let mut lag = min_lag;
                while lag < max_lag {
                    let mut acc = 0.0f32;
                    let mut e1 = 0.0f32;
                    for i in 0..(N - max_lag) {
                        acc += scratch[i] * scratch[i + lag];
                        e1 += scratch[i + lag] * scratch[i + lag];
                    }
                    let norm = (e0 * e1).sqrt().max(1e-9);
                    let r = acc / norm;
                    if r > best {
                        best = r;
                        best_lag = lag;
                    }
                    lag += 1;
                }
                if best > 0.6 && best_lag > 0 {
                    pitch = sr / best_lag as f32;
                }
            }
            shared.pitch_hz.store(pitch.to_bits(), Ordering::Relaxed);
            SignalToUI::set_ui_signal();
        });
    }

    /// Sensitivity drives both the note detector and the presence gate/duck.
    fn set_voice_threshold(&mut self, th: f32) {
        self.voice.threshold = th;
        let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
        granular.settings.gate_thresh.set(th);
    }

    fn set_voice_enabled(&mut self, cx: &mut Cx, on: bool) {
        self.voice.enabled = on;
        self.ui.check_box_set(ids!(voice.checkbox)).set_active(cx, on);
        if !on {
            self.voice_note_off(cx);
        }
        self.update_voice_portraits(cx);
    }

    fn voice_note_on(&mut self, cx: &mut Cx, note: u8) {
        self.voice_note_off(cx);
        self.audio_graph.send_midi_data(
            MidiNote { channel: 0, is_on: true, note_number: note, velocity: 100 }.into(),
        );
        self.ui.piano_set(ids!(piano)).set_note(cx, true, note);
        self.voice.active_note = Some(note);
        self.update_voice_portraits(cx);
    }

    fn voice_note_off(&mut self, cx: &mut Cx) {
        if let Some(note) = self.voice.active_note.take() {
            self.audio_graph.send_midi_data(
                MidiNote { channel: 0, is_on: false, note_number: note, velocity: 0 }.into(),
            );
            self.ui.piano_set(ids!(piano)).set_note(cx, false, note);
            self.update_voice_portraits(cx);
        }
    }

    fn update_voice_portraits(&mut self, cx: &mut Cx) {
        let singing = self.voice.active_note.is_some();
        self.ui.view(id!(portrait_cat)).set_visible(cx, !singing);
        self.ui.view(id!(portrait_roy)).set_visible(cx, singing);
    }

    fn voice_update(&mut self, cx: &mut Cx) {
        let level = self.voice.shared.level();
        let pitch = self.voice.shared.pitch();
        self.voice.smoothed_level += (level - self.voice.smoothed_level) * 0.3;
        let now = cx.seconds_since_app_start();
        self.ui.mandelbrot(id!(fractal)).set_voice_level((self.voice.smoothed_level / self.voice.threshold.max(1e-3)).min(3.0) / 3.0);
        if !self.voice.enabled {
            return;
        }

        let th = self.voice.threshold;
        if level > th && pitch > 0.0 {
            self.voice.silence_since = None;
            let midi = (69.0 + 12.0 * (pitch / 440.0).log2()).round().clamp(36.0, 84.0) as u8;
            let retrigger = match self.voice.active_note {
                None => true,
                Some(_) => (midi as i32 - self.voice.base_note as i32).abs() > 1,
            };
            if retrigger {
                let seed = &mut self.voice.seed;
                // random interval jump, weighted toward none
                let r = next_u32(seed) % 10;
                self.voice.interval = match r {
                    0 => 12,
                    1 => -12,
                    2 => 7,
                    3 => -5,
                    4 => 5,
                    _ => 0,
                };
                self.voice.base_note = midi;
                let note = (midi as i32 + self.voice.interval as i32).clamp(24, 96) as u8;
                self.voice_note_on(cx, note);
                // seed the Life grid at a pitch-dependent row
                let row = ((midi.saturating_sub(36)) as usize * 16 / 49).min(15);
                let count = if level > th * 2.0 { 2 } else { 1 };
                self.life_stamp_at_row(cx, row, count);
            }
        } else if level < th * 0.5 {
            match self.voice.silence_since {
                None => self.voice.silence_since = Some(now),
                Some(t) if now - t > 0.15 => {
                    self.voice_note_off(cx);
                }
                _ => {}
            }
        }

        // while a note is held: rapidly randomize the synth
        if self.voice.active_note.is_some() && now - self.voice.last_rand_time > 0.06 {
            self.voice.last_rand_time = now;
            let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
            ironfish.settings.randomize(&mut self.voice.seed, self.voice.chaos);
            self.init_ui_state(cx);
        }
    }
}
impl App{
    #[cfg(target_arch = "wasm32")]
    pub fn preset(&mut self, _cx: &mut Cx, _index: usize, _save: bool) {
        
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    pub fn preset(&mut self, cx: &mut Cx, index: usize, save: bool) {
        use std::fs::File;
        use std::io::prelude::*;
        
        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        
        let file_name = format!("{}/preset_{}.txt", env!("CARGO_MANIFEST_DIR"), index);
        if save {
            let nodes = ironfish.settings.live_read();
            let data = nodes.to_cbor(0).unwrap();
            let data = makepad_miniz::compress_to_vec(&data, 10);
            let data = makepad_base64::base64_encode(&data, &makepad_base64::BASE64_URL_SAFE);
            log!("Saving preset {}", file_name);
            let mut file = File::create(&file_name).unwrap();
            file.write_all(&data).unwrap();
        }
        else if let Ok(mut file) = std::fs::File::open(&file_name) {
            log!("Loading preset {}", file_name);
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if let Ok(data) = makepad_base64::base64_decode(&data) {
                if let Ok(data) = makepad_miniz::decompress_to_vec(&data) {
                    let mut nodes = Vec::new();
                    nodes.from_cbor(&data).unwrap();
                    ironfish.settings.apply_over(cx, &nodes);
                    self.init_ui_state(cx);
                }
                else {
                    log!("Error decompressing preset");
                }
            }
            else {
                log!("Error base64 decoding preset");
            }
        }
    }
}

// Game of Life
impl App {
    fn life_grid(&mut self) -> Life {
        let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
        let steps: Vec<u32> = (0..16).map(|i| granular.settings.steps[i].get()).collect();
        Life::from_steps(&steps)
    }

    /// Writes a grid to the grain sequencer model and every on-screen sequencer.
    fn write_life_grid(&mut self, cx: &mut Cx, life: &Life) {
        let steps = life.to_steps();
        let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
        for (i, s) in steps.iter().enumerate() {
            granular.settings.steps[i].set(*s);
        }
        self.ui.sequencer_set(ids!(sequencer)).set_steps(cx, &steps);
        self.ui.redraw(cx);
    }

    fn set_life_enabled(&mut self, cx: &mut Cx, on: bool) {
        self.life.enabled = on;
        self.ui.check_box_set(ids!(life.checkbox)).set_active(cx, on);
        let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
        granular.settings.life.set(on);
        if on {
            if self.life_grid().is_empty() {
                self.life_seed(cx);
            }
            // generations only happen while the delay runs
            let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
            granular.settings.running.set(true);
            self.ui.check_box_set(ids!(playpause.checkbox)).set_active(cx, true);
        }
    }

    /// Stamp a random pattern at a random position.
    fn life_seed(&mut self, cx: &mut Cx) {
        let mut grid = self.life_grid();
        let seed = &mut self.life.seed;
        let n = 1 + next_u32(seed) % 3;
        for _ in 0..n {
            let p = Pattern::pick(seed);
            let x = (next_u32(seed) % W as u32) as usize;
            let y = (next_u32(seed) % H as u32) as usize;
            grid.stamp(p, x, y, seed);
        }
        self.write_life_grid(cx, &grid);
    }

    /// Stamp a pattern at a chosen row, starting at the playhead so the next
    /// slots of input get recorded into it (used by voice control).
    pub fn life_stamp_at_row(&mut self, cx: &mut Cx, row: usize, count: usize) {
        let slot = self.audio_graph.by_type::<GranularDelay>().unwrap().current_slot();
        let mut grid = self.life_grid();
        let seed = &mut self.life.seed;
        for i in 0..count {
            let p = Pattern::pick(seed);
            let x = (slot + 1 + i * 4) % W;
            grid.stamp(p, x, row.min(H - 1), seed);
        }
        self.write_life_grid(cx, &grid);
    }

    fn set_life_mode(&mut self, cx: &mut Cx, fractal: bool) {
        self.ui.view(id!(grid_page)).set_visible(cx, !fractal);
        self.ui.view(id!(fractal_page)).set_visible(cx, fractal);
        self.ui.mandelbrot(id!(fractal)).set_running(cx, fractal);
        self.ui.redraw(cx);
    }
}

// Scripted audio QA (MANGUO_AUDIO_QA=1 with MANGUO_RECORD_WAV=<dir> and
// MANGUO_VOICE_TEST=262:3:8): drives the grain delay through its modes with a
// fixed grid while the node records input / repeats / output stems.
impl App {
    fn aqa_grid(&mut self, cx: &mut Cx, cells: &[(usize, usize)]) {
        let mut grid = Life::default();
        for (x, y) in cells {
            grid.set(*x, *y, true);
        }
        self.write_life_grid(cx, &grid);
    }

    fn aqa_step(&mut self, cx: &mut Cx) {
        let step = self.life.qa_step;
        self.life.qa_step += 1;
        let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
        granular.shared.marker.store(step as u32 + 1, Ordering::Relaxed);
        let g = granular.settings.clone();
        let next;
        match step {
            0 => {
                log!("AQA: A digital, time 0.5, repeats 0.7, life off");
                g.mode.set(DelayMode::Digital);
                g.time.set(0.5);
                g.repeats.set(0.7);
                g.size.set(2.0);
                g.mix.set(1.0);
                g.pitch.set(1.0);
                g.reverse.set(0.0);
                g.spray.set(0.0);
                g.density.set(1.0);
                g.width.set(0.5);
                g.tone.set(0.0);
                g.duck.set(0.0);
                g.freeze.set(false);
                g.gate.set(false);
                g.life.set(false);
                g.running.set(true);
                next = 1.5; // t = 2.0, inside burst 1 (0..3)
            }
            1 => {
                log!("AQA: A stamp unison row 8, columns 4..6");
                self.aqa_grid(cx, &[(4, 8), (5, 8), (6, 8)]);
                next = 5.0; // t = 7.0
            }
            2 => {
                log!("AQA: B analog, repeats 0.95 (self-oscillation)");
                self.aqa_grid(cx, &[]);
                g.mode.set(DelayMode::Analog);
                g.repeats.set(0.95);
                next = 1.5; // t = 8.5, inside burst 2 (8..11)
            }
            3 => {
                log!("AQA: B stamp");
                self.aqa_grid(cx, &[(4, 8), (5, 8), (6, 8)]);
                next = 6.5; // t = 15.0
            }
            4 => {
                log!("AQA: C tape, repeats 0.6");
                self.aqa_grid(cx, &[]);
                g.mode.set(DelayMode::Tape);
                g.repeats.set(0.6);
                next = 1.5; // t = 16.5, inside burst 3 (16..19)
            }
            5 => {
                log!("AQA: C stamp");
                self.aqa_grid(cx, &[(4, 8), (5, 8), (6, 8)]);
                next = 6.5; // t = 23.0
            }
            6 => {
                log!("AQA: D digital, repeats 0.8, life on");
                self.aqa_grid(cx, &[]);
                g.mode.set(DelayMode::Digital);
                g.repeats.set(0.8);
                g.life.set(true);
                self.life.enabled = true;
                next = 1.5; // t = 24.5, inside burst 4 (24..27)
            }
            7 => {
                log!("AQA: D stamp glider + blinker");
                self.aqa_grid(cx, &[(4, 7), (5, 8), (3, 9), (4, 9), (5, 9), (13, 2), (13, 3), (13, 4)]);
                next = 7.0; // t = 31.5
            }
            8 => {
                log!("AQA: E gate: grid clear, life off, synth note held, gate on (release 0.1)");
                self.aqa_grid(cx, &[]);
                g.life.set(false);
                self.life.enabled = false;
                g.gate.set(true);
                g.gate_release.set(0.1);
                g.synth_gain.set(1.0);
                self.audio_graph.send_midi_data(MidiNote { channel: 0, is_on: true, note_number: 60, velocity: 100 }.into());
                next = 8.0; // t = 39.5, covers burst 5 (32..35) and the gap after it
            }
            9 => {
                log!("AQA: F duck 1.0: note off, gate off, digital repeats 1.0");
                self.audio_graph.send_midi_data(MidiNote { channel: 0, is_on: false, note_number: 60, velocity: 0 }.into());
                g.gate.set(false);
                g.duck.set(1.0);
                g.mode.set(DelayMode::Digital);
                g.repeats.set(1.0);
                next = 1.0; // t = 40.5, inside burst 6 (40..43)
            }
            10 => {
                log!("AQA: F stamp");
                self.aqa_grid(cx, &[(4, 8), (5, 8), (6, 8)]);
                next = 10.0; // t = 50.5: ducked 41..43, open 43..48, ducked 48..51
            }
            11 => {
                log!("AQA: G dry/wet: grid clear, duck 0, mix 0 (dry voice only)");
                self.aqa_grid(cx, &[]);
                g.duck.set(0.0);
                g.mix.set(0.0);
                next = 7.0; // t = 57.5, inside burst 8 (56..59)
            }
            12 => {
                log!("AQA: G mix 1 (dry gone)");
                g.mix.set(1.0);
                next = 1.0; // t = 58.5, still inside burst 8
            }
            13 => {
                log!("AQA: H freeze: digital repeats 0.5");
                g.repeats.set(0.5);
                next = 6.0; // t = 64.5, inside burst 9 (64..67)
            }
            14 => {
                log!("AQA: H stamp");
                self.aqa_grid(cx, &[(4, 8), (5, 8), (6, 8)]);
                next = 1.2; // t = 65.7: cells recorded, first repeat played
            }
            15 => {
                log!("AQA: H freeze on");
                g.freeze.set(true);
                next = 6.0; // t = 71.7
            }
            16 => {
                log!("AQA: I reverse 1.0: freeze off, grid clear, repeats 0.7");
                g.freeze.set(false);
                self.aqa_grid(cx, &[]);
                g.reverse.set(1.0);
                g.repeats.set(0.7);
                next = 0.8; // t = 72.5, inside burst 10 (72..75)
            }
            17 => {
                log!("AQA: I stamp");
                self.aqa_grid(cx, &[(4, 8), (5, 8), (6, 8)]);
                next = 5.0; // t = 77.5
            }
            18 => {
                log!("AQA: stop recording");
                granular.shared.record_stop.store(1, Ordering::Relaxed);
                next = 1.0;
            }
            _ => {
                let frames = granular.shared.frames.load(Ordering::Relaxed);
                log!(
                    "AQA: perf max_render_us={} frames={} buffer_us={} underruns={} written={}",
                    granular.shared.max_render_us.load(Ordering::Relaxed),
                    frames,
                    frames as f64 / 48000.0 * 1e6,
                    granular.shared.mic_underruns.load(Ordering::Relaxed),
                    granular.shared.record_stop.load(Ordering::Relaxed) == 2
                );
                log!("AQA: done");
                std::process::exit(0);
            }
        }
        self.qa_schedule(cx, next);
    }
}

// Scripted QA run (MANGUO_QA=1): walks the app through its states on a timer,
// logging each step; pair with MAKEPAD_GRAB_SERIES for frame evidence.
impl App {
    fn qa_schedule(&mut self, cx: &mut Cx, secs: f64) {
        self.life.qa_timer = cx.start_timeout(secs);
    }

    fn qa_step(&mut self, cx: &mut Cx) {
        let ui = self.ui.clone();
        let step = self.life.qa_step;
        self.life.qa_step += 1;
        let next;
        match step {
            0..=7 => {
                log!("QA: preset {}", step);
                self.preset(cx, step, false);
                next = 0.35;
            }
            8 => {
                log!("QA: voice on (test tone runs for its configured seconds)");
                self.set_voice_enabled(cx, true);
                next = 3.0;
            }
            9 => {
                log!("QA: life on + window open (grid)");
                self.set_life_enabled(cx, true);
                ui.window(id!(life_window)).open(cx);
                self.set_life_mode(cx, false);
                next = 2.0;
            }
            10 => {
                log!("QA: fractal mode");
                self.set_life_mode(cx, true);
                next = 4.0;
            }
            11..=30 => {
                // stress: close / reopen the life window rapidly
                let open = step % 2 == 0;
                log!("QA: life window {}", if open { "open" } else { "close" });
                if open {
                    ui.window(id!(life_window)).open(cx);
                } else {
                    ui.window(id!(life_window)).close(cx);
                }
                next = 0.3;
            }
            31 => {
                log!("QA: back to grid mode, window open");
                ui.window(id!(life_window)).open(cx);
                self.set_life_mode(cx, false);
                next = 1.0;
            }
            32..=37 => {
                // granular parameter sweep
                let granular = self.audio_graph.by_type::<GranularDelay>().unwrap();
                let t = (step - 32) as f32 / 5.0;
                granular.settings.time.set(0.1 + 1.9 * t);
                granular.settings.repeats.set(t);
                granular.settings.size.set(1.0 + 3.0 * t);
                granular.settings.mix.set(0.3 + 0.7 * t);
                granular.settings.pitch.set(1.0 - t);
                granular.settings.reverse.set(t);
                granular.settings.spray.set(t);
                granular.settings.density.set(1.0 - 0.7 * t);
                granular.settings.width.set(t);
                granular.settings.tone.set(2.0 * t - 1.0);
                granular.settings.duck.set(t);
                granular.settings.freeze.set(step % 2 == 1);
                granular.settings.gate.set(step % 2 == 0);
                granular.settings.gate_release.set(0.03 + 2.0 * t);
                granular.settings.mode.set(match step % 3 { 0 => DelayMode::Digital, 1 => DelayMode::Analog, _ => DelayMode::Tape });
                log!("QA: delay sweep t={:.1} mode {:?}", t, granular.settings.mode.get());
                self.init_ui_state(cx);
                next = 1.2;
            }
            38 => {
                log!("QA: voice off, life off, window closed");
                self.set_voice_enabled(cx, false);
                self.set_life_enabled(cx, false);
                ui.window(id!(life_window)).close(cx);
                next = 1.0;
            }
            _ => {
                log!("QA: done");
                std::process::exit(0);
            }
        }
        self.qa_schedule(cx, next);
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        if let Event::Timer(te) = event {
            if self.life.qa_timer.is_timer(te).is_some() {
                if self.life.audio_qa {
                    self.aqa_step(cx);
                } else {
                    self.qa_step(cx);
                }
            }
            if self.life.test_close_timer.is_timer(te).is_some() {
                cx.stop_timer(self.life.test_close_timer);
                log!("MANGUO_CLOSE_LIFE_AFTER: closing life window");
                self.ui.window(id!(life_window)).close(cx);
            }
        }
        self.ui.handle_event(cx, event, &mut Scope::empty());

        self.audio_graph
            .handle_event_with(cx, event, &mut |cx, action| {
                let display_audio = self.ui.display_audio_set(ids!(display_audio));
                match action {
                    AudioGraphAction::DisplayAudio { buffer, voice, .. } => {
                        display_audio.process_buffer(cx, None, voice, buffer, 1.0);
                    }
                    AudioGraphAction::VoiceOff { voice } => {
                        display_audio.voice_off(cx, voice);
                    }
                };
            });
    }
}
