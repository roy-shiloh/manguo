use crate::makepad_platform::live_atomic::AtomicGetSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use crate::{
    life::*, life_fractal::*, makepad_audio_graph::*, makepad_audio_widgets::display_audio::*,
    makepad_audio_widgets::piano::*, makepad_synth_ironfish::ironfish::*, makepad_widgets::*,
    sequencer::*,
};

live_design! {
    use link::widgets::*
    use link::theme::*
    use makepad_example_ironfish::app_desktop::AppDesktop
    use makepad_example_ironfish::app_desktop::LifeWindow

    use makepad_audio_graph::mixer::Mixer;
    use makepad_audio_graph::instrument::Instrument;
    use makepad_synth_ironfish::ironfish::IronFish;

    App = {{App}} {

        audio_graph: {
            root: <Mixer> {
                c1 = <Instrument> {
                    <IronFish> {}
                }
            }
        }
        ui: <Root>{
            main_window = <Window> {
                window: {inner_size: vec2(1280, 1060), title: "IRONFISH 芒果"},
                pass: {clear_color: #x1A0A14}
                block_signal_event: true;
                body = <AppDesktop> {}
            }
            life_window = <Window> {
                window: {inner_size: vec2(720, 800), title: "IRONFISH LIFE", open_on_start: false},
                pass: {clear_color: #x1A0A14}
                block_signal_event: true;
                body = <LifeWindow> {}
            }
        }
    }
}
app_main!(App);

/// Published by the audio-input callback, read on the UI thread. Lock-free.
#[derive(Default)]
pub struct VoiceShared {
    /// RMS level of the last input buffer, as f32 bits
    level: AtomicU32,
    /// Estimated fundamental in Hz (0 when unsure), as f32 bits
    pitch_hz: AtomicU32,
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

/// Game of Life running inside the sequencer grid.
#[derive(Default)]
pub struct LifeControl {
    pub enabled: bool,
    /// Sequencer steps per Life generation (1..=16).
    pub steps_per_gen: u32,
    pub timer: Timer,
    pub seed: u32,
    pub generation: u64,
    /// dev: one-shot timer that closes the life window (IRONFISH_CLOSE_LIFE_AFTER)
    pub test_close_timer: Timer,
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    audio_graph: AudioGraph,
    #[rust]
    midi_input: MidiInput,
    #[rust(LifeControl { steps_per_gen: 4, seed: 0xC0FFEE, ..Default::default() })]
    life: LifeControl,
    #[rust]
    voice: VoiceControl,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        crate::makepad_audio_widgets::live_design(cx);
        crate::makepad_audio_graph::live_design(cx);
        crate::makepad_synth_ironfish::live_design(cx);
        crate::sequencer::live_design(cx);
        crate::life_fractal::live_design(cx);
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
    }
    
    pub fn data_bind(mut db: DataBindingMap) {
        // sequencer
        db.bind(id!(sequencer.playing), ids!(playpause));
        db.bind(id!(sequencer.bpm), ids!(speed.slider));
        db.bind(id!(sequencer.rootnote), ids!(rootnote.dropdown));
        db.bind(id!(sequencer.scale), ids!(scaletype.dropdown));
        db.bind(id!(arp.enabled), ids!(arp.checkbox));
        db.bind(id!(arp.octaves), ids!(arpoctaves.slider));

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
        //db.bind(id!(filter1.filter_type), ids!(filter_type.dropdown));
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

        // sequencer
        db.bind(id!(sequencer.steps), ids!(sequencer));

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
        self.ui.slider_set(ids!(liferate.slider)).set_value(cx, self.life.steps_per_gen as f64);
        self.start_voice_input(cx);
        // Dev/verification: IRONFISH_OPEN_LIFE=grid|fractal opens the life window running.
        if std::env::var("IRONFISH_VOICE_TEST").is_ok() {
            self.set_voice_enabled(cx, true);
        }
        if let Ok(mode) = std::env::var("IRONFISH_OPEN_LIFE") {
            self.ui.window(id!(life_window)).open(cx);
            self.set_life_enabled(cx, true);
            self.set_life_mode(cx, mode == "fractal");
        }
        if let Some(secs) = std::env::var("IRONFISH_CLOSE_LIFE_AFTER").ok().and_then(|v| v.parse::<f64>().ok()) {
            self.life.test_close_timer = cx.start_timeout(secs);
        }
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        
        let ui = self.ui.clone();
        let piano = ui.piano(id!(piano));

        ui.radio_button_set(ids!(oscillators.tab1, oscillators.tab2,))
            .selected_to_visible(cx, &ui, actions, ids!(oscillators.osc1, oscillators.osc2,));

        ui.radio_button_set(ids!(filter_modes.tab1, filter_modes.tab2,))
            .selected_to_visible(
                cx,
                &ui,
                actions,
                ids!(preset_pages.tab1_frame, preset_pages.tab2_frame,),
            );

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

        if ui.button_set(ids!(panic)).clicked(&actions) {
            cx.midi_reset();
            self.audio_graph.all_notes_off();
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
            self.voice.threshold = 0.15 * (0.005f32 / 0.15).powf(v as f32);
        }
        if let Some(v) = ui.slider(id!(voicechaos.slider)).slided(&actions) {
            self.voice.chaos = v as f32;
        }

        // Game of Life controls (present in both windows; ids are shared)
        let life_toggled = ui.check_box_set(ids!(life.checkbox)).changed(&actions);
        if let Some(on) = life_toggled {
            self.set_life_enabled(cx, on);
        }
        if let Some(v) = ui.slider_set(ids!(liferate.slider)).slided(&actions) {
            self.life.steps_per_gen = (v.round() as u32).clamp(1, 16);
            ui.slider_set(ids!(liferate.slider)).set_value(cx, self.life.steps_per_gen as f64);
            if self.life.enabled {
                self.restart_life_timer(cx);
            }
        }
        if ui.button_set(ids!(lifeseed.button)).clicked(&actions) {
            self.life_seed(cx);
        }
        if ui.button_set(ids!(lifewindow.button)).clicked(&actions) {
            ui.window(id!(life_window)).open(cx);
            // make sure the fractal has the current grid
            let grid = self.life_grid();
            ui.life_fractal(id!(fractal)).set_grid(cx, &grid);
        }
        if ui.button(id!(tab_grid)).clicked(&actions) {
            self.set_life_mode(cx, false);
        }
        if ui.button(id!(tab_fractal)).clicked(&actions) {
            self.set_life_mode(cx, true);
        }
        if let Some(v) = ui.slider(id!(zoomspeed.slider)).slided(&actions) {
            ui.life_fractal(id!(fractal)).set_speed(v as f32);
        }

        let mut db = DataBindingStore::new();
        db.data_bind(cx, actions, &ui, Self::data_bind);

        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        ironfish.settings.apply_over(cx, &db.nodes);

        // bpm changed while Life runs: retime the generation clock
        if self.life.enabled && ui.slider_set(ids!(speed.slider)).slided(&actions).is_some() {
            self.restart_life_timer(cx);
        }
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
        // Dev/verification: IRONFISH_VOICE_TEST=<hz>[:<seconds>] replaces the
        // mic with a synthetic "sung" tone (optionally stopping after N seconds)
        // so the whole chain can be exercised headless.
        let (test_hz, test_secs): (Option<f32>, f32) = match std::env::var("IRONFISH_VOICE_TEST") {
            Ok(v) => {
                let mut it = v.split(':');
                (it.next().and_then(|h| h.parse().ok()), it.next().and_then(|s| s.parse().ok()).unwrap_or(f32::INFINITY))
            }
            Err(_) => (None, 0.0),
        };
        let mut phase = 0.0f32;
        let mut test_frames = 0u64;
        cx.audio_input(0, move |_info, input| {
            let frames = input.frame_count();
            if frames == 0 || input.channel_count() == 0 {
                return;
            }
            let ch = input.channel(0);
            let mut sum = 0.0f32;
            for i in 0..frames {
                let mut s = ch[i];
                if let Some(hz) = test_hz {
                    test_frames += 1;
                    phase = (phase + hz / 48000.0).fract();
                    s = if (test_frames as f32) < test_secs * 48000.0 {
                        0.2 * (phase * std::f32::consts::TAU).sin() + 0.08 * (phase * 2.0 * std::f32::consts::TAU).sin()
                    } else { 0.0 };
                }
                sum += s * s;
                ring[write] = s;
                write = (write + 1) % N;
            }
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
        self.ui.life_fractal(id!(fractal)).set_voice_level((self.voice.smoothed_level / self.voice.threshold.max(1e-3)).min(3.0) / 3.0);
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
        
        let file_name = format!("examples/ironfish/preset_{}.txt", index);
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
        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        let steps: Vec<u32> = (0..16).map(|i| ironfish.settings.sequencer.get_step(i)).collect();
        Life::from_steps(&steps)
    }

    /// Writes a grid to the synth model and every on-screen sequencer.
    fn write_life_grid(&mut self, cx: &mut Cx, life: &Life) {
        let steps = life.to_steps();
        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        for (i, s) in steps.iter().enumerate() {
            ironfish.settings.sequencer.set_step(i, *s);
        }
        self.ui.sequencer_set(ids!(sequencer)).set_steps(cx, &steps);
        self.ui.life_fractal(id!(fractal)).set_grid(cx, life);
        self.ui.redraw(cx);
    }

    fn life_interval(&mut self) -> f64 {
        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        let bpm = ironfish.settings.sequencer.bpm.get().max(20.0) as f64;
        // one sequencer step is a 16th note
        60.0 / bpm / 4.0 * self.life.steps_per_gen.max(1) as f64
    }

    fn restart_life_timer(&mut self, cx: &mut Cx) {
        cx.stop_timer(self.life.timer);
        self.life.timer = cx.start_interval(self.life_interval());
    }

    fn set_life_enabled(&mut self, cx: &mut Cx, on: bool) {
        self.life.enabled = on;
        self.ui.check_box_set(ids!(life.checkbox)).set_active(cx, on);
        if on {
            if self.life_grid().is_empty() {
                self.life_seed(cx);
            }
            // make the cells audible
            let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
            ironfish.settings.sequencer.playing.set(true);
            self.ui.check_box_set(ids!(playpause)).set_active(cx, true);
            self.restart_life_timer(cx);
        } else {
            cx.stop_timer(self.life.timer);
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

    /// Stamp a pattern at a chosen row (used by voice control).
    pub fn life_stamp_at_row(&mut self, cx: &mut Cx, row: usize, count: usize) {
        let mut grid = self.life_grid();
        let seed = &mut self.life.seed;
        for _ in 0..count {
            let p = Pattern::pick(seed);
            let x = (next_u32(seed) % W as u32) as usize;
            grid.stamp(p, x, row.min(H - 1), seed);
        }
        self.write_life_grid(cx, &grid);
    }

    fn set_life_mode(&mut self, cx: &mut Cx, fractal: bool) {
        self.ui.view(id!(grid_page)).set_visible(cx, !fractal);
        self.ui.view(id!(fractal_page)).set_visible(cx, fractal);
        let grid = self.life_grid();
        let f = self.ui.life_fractal(id!(fractal));
        f.set_grid(cx, &grid);
        f.set_running(cx, fractal);
        self.ui.redraw(cx);
    }

    fn life_tick(&mut self, cx: &mut Cx) {
        let mut next = self.life_grid().step();
        if next.is_empty() {
            let seed = &mut self.life.seed;
            let p = Pattern::pick(seed);
            let x = (next_u32(seed) % W as u32) as usize;
            let y = (next_u32(seed) % H as u32) as usize;
            next.stamp(p, x, y, seed);
        }
        self.life.generation += 1;
        self.write_life_grid(cx, &next);
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        if let Event::Timer(te) = event {
            if self.life.timer.is_timer(te).is_some() && self.life.enabled {
                self.life_tick(cx);
            }
            if self.life.test_close_timer.is_timer(te).is_some() {
                cx.stop_timer(self.life.test_close_timer);
                log!("IRONFISH_CLOSE_LIFE_AFTER: closing life window");
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
