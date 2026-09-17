use crate::{
    makepad_audio_graph::*, makepad_audio_widgets::display_audio::*,
    makepad_audio_widgets::piano::*, makepad_synth_ironfish::ironfish::*, makepad_widgets::*,
    sequencer::*,
};

live_design! {
    use link::widgets::*
    use link::theme::*
    use makepad_example_ironfish::app_desktop::AppDesktop

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
                window: {inner_size: vec2(1280, 1000)},
                pass: {clear_color: #2A}
                block_signal_event: true;
                body = <AppDesktop> {}
            }
        }
    }
}
app_main!(App);

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    audio_graph: AudioGraph,
    #[rust]
    midi_input: MidiInput,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        crate::makepad_audio_widgets::live_design(cx);
        crate::makepad_audio_graph::live_design(cx);
        crate::makepad_synth_ironfish::live_design(cx);
        crate::sequencer::live_design(cx);
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
        
        let mut db = DataBindingStore::new();
        db.data_bind(cx, actions, &ui, Self::data_bind);


        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        ironfish.settings.apply_over(cx, &db.nodes);
    }

    fn handle_midi_ports(&mut self, cx: &mut Cx, ports: &MidiPortsEvent) {
        cx.use_midi_inputs(&ports.all_inputs());
    }

    fn handle_audio_devices(&mut self, cx: &mut Cx, devices: &AudioDevicesEvent) {
        cx.use_audio_outputs(&devices.default_output());
    }

    fn handle_signal(&mut self, cx: &mut Cx) {
        let piano = self.ui.piano_set(ids!(piano));
        while let Some((_, data)) = self.midi_input.receive() {
            self.audio_graph.send_midi_data(data);
            if let Some(note) = data.decode().on_note() {
                piano.set_note(cx, note.is_on, note.note_number)
            }
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

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
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
