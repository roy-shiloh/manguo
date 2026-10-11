//! Manguo Grain: the Game-of-Life granular delay from Manguo as a VST3/CLAP
//! plugin, with its makepad UI rendered inside the host's editor window.

pub use makepad_widgets;

pub mod app;
pub mod editor;
pub mod grain_core;
pub mod life;
pub mod sequencer;

use grain_core::{DelayMode, Engine, EngineParams, GrainScale, GridShared};
use nice_plug::params::persist::PersistentField;
use nice_plug::prelude::*;
use std::sync::Arc;

/// The grid as plugin state: persisted with the preset, shared with the editor
/// and the engine through atomics.
pub struct GridState(pub Arc<GridShared>);

impl<'a> PersistentField<'a, [u32; 16]> for GridState {
    fn set(&self, new_value: [u32; 16]) {
        self.0.set_steps(&new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&[u32; 16]) -> R,
    {
        f(&self.0.steps())
    }
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeParam {
    Digital,
    Analog,
    Tape,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScaleParam {
    Minor,
    Major,
    Dorian,
    Pentatonic,
}

#[derive(Params)]
pub struct GrainParams {
    #[id = "run"]
    pub running: BoolParam,
    #[id = "life"]
    pub life: BoolParam,
    #[id = "freeze"]
    pub freeze: BoolParam,
    #[id = "mode"]
    pub mode: EnumParam<ModeParam>,
    #[id = "scale"]
    pub scale: EnumParam<ScaleParam>,
    #[id = "time"]
    pub time: FloatParam,
    #[id = "repeats"]
    pub repeats: FloatParam,
    #[id = "size"]
    pub size: FloatParam,
    #[id = "pitch"]
    pub pitch: FloatParam,
    #[id = "reverse"]
    pub reverse: FloatParam,
    #[id = "spray"]
    pub spray: FloatParam,
    #[id = "density"]
    pub density: FloatParam,
    #[id = "width"]
    pub width: FloatParam,
    #[id = "tone"]
    pub tone: FloatParam,
    #[id = "duck"]
    pub duck: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "input"]
    pub input: FloatParam,
    #[id = "sens"]
    pub sensitivity: FloatParam,
    #[id = "release"]
    pub gate_release: FloatParam,
    #[id = "stamp"]
    pub auto_stamp: BoolParam,

    #[persist = "grid"]
    pub grid: GridState,
}

impl GrainParams {
    pub fn new(shared: Arc<GridShared>) -> Self {
        let unit = |name: &str, default: f32| {
            FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2))
        };
        Self {
            running: BoolParam::new("Run", true),
            life: BoolParam::new("Life", false),
            freeze: BoolParam::new("Freeze", false),
            mode: EnumParam::new("Mode", ModeParam::Digital),
            scale: EnumParam::new("Scale", ScaleParam::Minor),
            time: FloatParam::new(
                "Time",
                0.5,
                FloatRange::Skewed { min: 0.1, max: 2.0, factor: FloatRange::skew_factor(-1.0) },
            )
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            repeats: unit("Repeats", 0.6),
            size: FloatParam::new("Size", 2.0, FloatRange::Linear { min: 1.0, max: 4.0 })
                .with_unit(" slots")
                .with_value_to_string(formatters::v2s_f32_rounded(1)),
            pitch: unit("Pitch", 1.0),
            reverse: unit("Reverse", 0.0),
            spray: unit("Spray", 0.0),
            density: unit("Density", 1.0),
            width: unit("Width", 0.5),
            tone: FloatParam::new("Tone", 0.0, FloatRange::Linear { min: -1.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            duck: unit("Duck", 0.0),
            mix: unit("Dry/Wet", 0.5),
            input: FloatParam::new("Input", 1.0, FloatRange::Linear { min: 0.0, max: 4.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            sensitivity: unit("Sensitivity", 0.5),
            gate_release: FloatParam::new("Gate Release", 0.3, FloatRange::Linear { min: 0.03, max: 2.0 })
                .with_unit(" s")
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            auto_stamp: BoolParam::new("Stamp", true),
            grid: GridState(shared),
        }
    }

    /// Sensitivity 0..1 -> input RMS threshold 0.15 .. 0.005 (log-ish);
    /// at 0 the gate is closed.
    pub fn threshold(sensitivity: f32) -> f32 {
        if sensitivity <= 0.001 {
            return 10.0;
        }
        0.15 * (0.005f32 / 0.15).powf(sensitivity)
    }

    pub fn snapshot(&self) -> EngineParams {
        EngineParams {
            running: self.running.value(),
            life: self.life.value(),
            freeze: self.freeze.value(),
            mode: match self.mode.value() {
                ModeParam::Digital => DelayMode::Digital,
                ModeParam::Analog => DelayMode::Analog,
                ModeParam::Tape => DelayMode::Tape,
            },
            scale: match self.scale.value() {
                ScaleParam::Minor => GrainScale::Minor,
                ScaleParam::Major => GrainScale::Major,
                ScaleParam::Dorian => GrainScale::Dorian,
                ScaleParam::Pentatonic => GrainScale::Pentatonic,
            },
            time: self.time.value(),
            repeats: self.repeats.value(),
            size: self.size.value(),
            pitch: self.pitch.value(),
            reverse: self.reverse.value(),
            spray: self.spray.value(),
            density: self.density.value(),
            width: self.width.value(),
            tone: self.tone.value(),
            duck: self.duck.value(),
            mix: self.mix.value(),
            input_gain: self.input.value(),
            gate_thresh: Self::threshold(self.sensitivity.value()),
            gate_release: self.gate_release.value(),
            auto_stamp: self.auto_stamp.value(),
        }
    }
}

pub struct ManguoGrain {
    params: Arc<GrainParams>,
    shared: Arc<GridShared>,
    engine: Engine,
    /// input copy: the host buffer is in-place
    scratch: Vec<Vec<f32>>,
}

impl Default for ManguoGrain {
    fn default() -> Self {
        let shared = Arc::new(GridShared::default());
        Self {
            params: Arc::new(GrainParams::new(shared.clone())),
            shared,
            engine: Engine::new(48000.0),
            scratch: Vec::new(),
        }
    }
}

impl Plugin for ManguoGrain {
    const NAME: &'static str = "Manguo Grain";
    const VENDOR: &'static str = "Manguo";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "info@makepad.nl";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type Editor = editor::GrainEditor;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Self::Editor> {
        Some(editor::GrainEditor::new(self.params.clone(), self.shared.clone()))
    }

    fn activate(
        &mut self,
        layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        self.engine.set_sample_rate(buffer_config.sample_rate);
        let channels = layout.main_input_channels.map(|c| c.get()).unwrap_or(0) as usize;
        let frames = buffer_config.max_buffer_size as usize;
        self.scratch = (0..channels.max(1)).map(|_| vec![0.0; frames]).collect();
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let frames = buffer.samples();
        let out = buffer.as_slice();
        if frames == 0 || out.is_empty() {
            return ProcessStatus::Normal;
        }
        if self.scratch.is_empty() || self.scratch[0].len() < frames {
            // host went past max_buffer_size: process without input rather than allocate
            return ProcessStatus::Normal;
        }
        let in_channels = out.len().min(self.scratch.len());
        for c in 0..in_channels {
            self.scratch[c][..frames].copy_from_slice(&out[c][..frames]);
        }
        let params = self.params.snapshot();
        let mut input: [&[f32]; 2] = [&[], &[]];
        for c in 0..in_channels.min(2) {
            input[c] = &self.scratch[c][..frames];
        }
        self.engine.process(&params, &self.shared, &input[..in_channels.min(2)], out);
        ProcessStatus::Normal
    }
}

impl ClapPlugin for ManguoGrain {
    const CLAP_ID: &'static str = "nl.makepad.manguo-grain";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Game-of-Life granular delay");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo, ClapFeature::Delay];
}

impl Vst3Plugin for ManguoGrain {
    const VST3_CLASS_ID: [u8; 16] = *b"ManguoGrainDelay";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Delay];
}

nice_export_clap!(ManguoGrain);
nice_export_vst3!(ManguoGrain);
