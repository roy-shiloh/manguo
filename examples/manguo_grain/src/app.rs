//! The plugin editor as a makepad app: the Manguo Grain Delay panel, bound
//! to the host parameters through nice-plug's `GuiContext`.

use crate::grain_core::GridShared;
use crate::sequencer::{SequencerSetWidgetRefExt, SequencerWidgetRefExt};
use crate::life::W;
use crate::makepad_widgets::*;
use crate::GrainParams;
use nice_plug::context::gui::GuiContext;
use nice_plug::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

pub const EDITOR_WIDTH: f64 = 640.0;
pub const EDITOR_HEIGHT: f64 = 640.0;

live_design! {
    use link::widgets::*
    use link::theme::*
    use link::shaders::*

    use manguo_grain::sequencer::Sequencer;

    FONT_SIZE_H2 = 6.5
    PX = 3.0

    SSPACING_0 = 0.0
    SSPACING_1 = 4.0
    SSPACING_2 = (SSPACING_1 * 2)
    SSPACING_3 = (SSPACING_1 * 3)
    SSPACING_4 = (SSPACING_1 * 4)

    SPACING_0 = {top: (SSPACING_0), right: (SSPACING_0), bottom: (SSPACING_0), left: (SSPACING_0)}
    SPACING_1 = {top: (SSPACING_1), right: (SSPACING_1), bottom: (SSPACING_1), left: (SSPACING_1)}
    SPACING_2 = {top: (SSPACING_2), right: (SSPACING_2), bottom: (SSPACING_2), left: (SSPACING_2)}

    PIXEL_FONT = <THEME_FONT_BOLD>{
        font_family: {
            latin = font("crate://self/resources/PressStart2P-Regular.ttf", 0.0, 0.0),
        },
        line_spacing: 1.4
    }

    H2_TEXT_BOLD = <PIXEL_FONT>{
        font_size: (FONT_SIZE_H2),
    }

    COLOR_BG = #x1A0A14
    COLOR_PANEL = #x2B1020
    COLOR_PANEL_2 = #x3A1530
    COLOR_HOT = #xFF3FA4
    COLOR_MAGENTA = #xE0218A
    COLOR_PALE = #xFFB3DA
    COLOR_HILITE = #xFFE1F0

    COLOR_DOWN_0 = #x0A020800
    COLOR_DOWN_3 = #x0A020866
    COLOR_DOWN_4 = #x0A020899
    COLOR_DOWN_5 = #x0A0208CC
    COLOR_DOWN_6 = #x0A0208EE

    COLOR_UP_1 = #xFFB3DA12
    COLOR_UP_2 = #xFFB3DA1E
    COLOR_UP_3 = #xFFB3DA30
    COLOR_UP_4 = #xFFB3DA55
    COLOR_UP_5 = #xFFB3DA99
    COLOR_UP_6 = #xFFB3DAE6

    COLOR_DEFAULT = (COLOR_UP_6)

    PX_CELL = dep("crate://self/resources/px/cell.png")
    PX_SEED = dep("crate://self/resources/px/seed.png")
    ICO_DOWN = dep("crate://self/resources/icons/Icon_Down.svg")
    ICO_SEQ_SWEEP = dep("crate://self/resources/icons/Icon_Seq_Sweep.svg")
    ICO_UP = dep("crate://self/resources/icons/Icon_Up.svg")

    PixelImage = <Image> {
        fit: Smallest
        draw_bg: {
            instance grid: 48.0
            instance tint: 1.0
            fn get_color(self) -> vec4 {
                let cell = floor(self.pos * self.grid);
                let uv = (cell + 0.5) / self.grid;
                let c = sample2d(self.image, uv * self.image_scale + self.image_pan);
                let q = floor(c.xyz * 5.0 + 0.5) / 5.0;
                let pinked = mix(q, q * vec3(1.15, 0.85, 1.05), self.tint * 0.6);
                return vec4(pinked, c.w);
            }
        }
    }

    PixelIcon = <PixelImage> {
        width: 24, height: 24
        draw_bg: { grid: 24.0, tint: 0.0 }
    }

    SquareView = <RoundedView> {
        draw_bg: { border_radius: 0.0 }
    }

    FillerH = <View> { width: Fill }

    ElementBox = <View> {
        draw_bg: {color: (COLOR_DOWN_0)}
        width: Fill,
        height: Fit
        flow: Down,
        padding: <SPACING_1> {}
        spacing: (SSPACING_1)
    }

    FishPanel = <GradientYView> {
        flow: Down,
        padding: <SPACING_2> {}
        width: Fill,
        height: Fit
        draw_bg: {
            instance border_width: 2.0
            instance inset: vec4(1.0, 1.0, 1.0, 1.0)
            color_1: (COLOR_PANEL_2),
            color_2: (COLOR_PANEL)
            instance bevel_light: (COLOR_UP_4)
            instance bevel_dark: (COLOR_DOWN_5)

            fn get_color(self) -> vec4 {
                let band = floor(self.pos.y * 6.0) / 6.0;
                return mix(self.color_1, self.color_2, band)
            }

            fn pixel(self) -> vec4 {
                let p = self.pos * self.rect_size;
                let bw = self.border_width;
                let x0 = self.inset.x;
                let y0 = self.inset.y;
                let x1 = self.rect_size.x - self.inset.z;
                let y1 = self.rect_size.y - self.inset.w;
                if (p.x < x0 || p.y < y0 || p.x >= x1 || p.y >= y1) {
                    return vec4(0.0);
                }
                let on_light = step(p.x, x0 + bw) + step(p.y, y0 + bw);
                let on_dark = step(x1 - bw, p.x) + step(y1 - bw, p.y);
                if on_dark > 0.0 {
                    return Pal::premul(self.bevel_dark);
                }
                if on_light > 0.0 {
                    return Pal::premul(self.bevel_light);
                }
                return Pal::premul(self.get_color());
            }
        }
    }

    FishDropDown = <DropDownFlat> {
        width: Fit
    }

    IconButton = <Button> {
        draw_icon: {
            svg_file: (ICO_SEQ_SWEEP),
            fn get_color(self) -> vec4 {
                return mix(mix((COLOR_UP_5), (COLOR_UP_6), self.hover), (COLOR_UP_4), self.down)
            }
        }
        icon_walk: {width: 7.5, height: Fit}
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                return sdf.result
            }
        }
        padding: 9.0
        text: ""
    }

    FishButton = <Button> {
        align: {x: 0.5, y: 0.5}
        padding: <SPACING_2> {}
        draw_text: {
            text_style: <H2_TEXT_BOLD> {}
            fn get_color(self) -> vec4 {
                return mix(mix((COLOR_UP_5), (COLOR_UP_6), self.hover), (COLOR_UP_5), self.down)
            }
        }
        draw_bg: {
            instance hover: 0.0
            instance down: 0.0
            fn pixel(self) -> vec4 {
                let p = self.pos * self.rect_size;
                let bw = 2.0;
                let light = mix((COLOR_UP_5), (COLOR_DOWN_5), self.down);
                let dark = mix((COLOR_DOWN_5), (COLOR_UP_5), self.down);
                let on_light = step(p.x, bw) + step(p.y, bw);
                let on_dark = step(self.rect_size.x - bw, p.x) + step(self.rect_size.y - bw, p.y);
                if on_dark > 0.0 {
                    return Pal::premul(dark);
                }
                if on_light > 0.0 {
                    return Pal::premul(light);
                }
                let fill = mix(mix((COLOR_PANEL_2), (COLOR_MAGENTA), self.hover * 0.5), (COLOR_HOT), self.down);
                return Pal::premul(fill);
            }
        }
    }

    FishSlider = <Slider> {
        margin: 0
        height: 36
        text: "CutOff1"
        draw_text: {text_style: <H2_TEXT_BOLD> {}, color: (COLOR_UP_5)}
        text_input: {
            empty_text: "0",
            is_numeric_only: true,
            draw_bg: { color: (COLOR_DOWN_0) },
        }
        draw_bg: {
            instance line_color: (COLOR_HOT)
            instance bipolar: 0.0
            fn pixel(self) -> vec4 {
                let px = (PX);
                let p = floor(self.pos * self.rect_size / px) * px;
                let top = 21.0;
                let w = self.rect_size.x;
                let h = self.rect_size.y;
                let sdf = Sdf2d::viewport(p);
                sdf.rect(px, top, w - 2.0 * px, h - top - px);
                sdf.fill_keep(mix((COLOR_DOWN_4), (COLOR_DOWN_6), self.drag));
                sdf.rect(px, top, w - 2.0 * px, px);
                sdf.fill((COLOR_DOWN_6));
                sdf.rect(px, h - 2.0 * px, w - 2.0 * px, px);
                sdf.fill((COLOR_UP_3));
                let inner = w - 4.0 * px;
                let origin = mix(2.0 * px, w * 0.5, self.bipolar);
                let nub_x = 2.0 * px + self.slide_pos * (inner - 3.0 * px);
                let bar_x0 = min(origin, nub_x);
                let bar_x1 = max(origin, nub_x + 3.0 * px);
                sdf.rect(bar_x0, top + 2.0 * px, bar_x1 - bar_x0, h - top - 4.0 * px);
                sdf.fill(mix(self.line_color * 0.8, self.line_color, self.hover + self.drag));
                sdf.rect(nub_x, top + px, 3.0 * px, h - top - 2.0 * px);
                sdf.fill_keep(mix((COLOR_PALE), (COLOR_HILITE), self.hover));
                sdf.rect(nub_x + 2.0 * px, top + px, px, h - top - 2.0 * px);
                sdf.fill((COLOR_MAGENTA));
                return sdf.result
            }
        }
    }

    InstrumentSlider = <ElementBox> {
        slider = <FishSlider> { draw_bg: {bipolar: 0.0} }
    }

    InstrumentBipolarSlider = <ElementBox> {
        slider = <FishSlider> { draw_bg: {bipolar: 1.0} }
    }

    FishToggle = <ElementBox> {
        padding: <SPACING_0> {}
        checkbox = <CheckBox> {
            width: 40,
            height: 30
            padding: {top: (SSPACING_0), right: (SSPACING_2), bottom: (SSPACING_0), left: 23}
            text: "CutOff1"
            animator: {
                active = {
                    default: off
                    off = {
                        from: {all: Forward {duration: 0.1}}
                        apply: {draw_bg: {active: 0.0}}
                    }
                    on = {
                        from: {all: Forward {duration: 0.1}}
                        apply: {draw_bg: {active: 1.0}}
                    }
                }
            }
            draw_bg: {
                size: 8.5;
                fn pixel(self) -> vec4 {
                    let px = (PX);
                    let sdf = Sdf2d::viewport(floor(self.pos * self.rect_size / px) * px)
                    let sz = self.size;
                    let left = px;
                    let cy = self.rect_size.y * 0.5;
                    let tw = sz * 3.0;
                    let th = sz * 2.0;
                    sdf.rect(left, cy - sz, tw, th);
                    sdf.fill(mix((COLOR_DOWN_5), (COLOR_MAGENTA), self.active));
                    sdf.rect(left, cy - sz, tw, px);
                    sdf.fill((COLOR_DOWN_6));
                    sdf.rect(left, cy + sz - px, tw, px);
                    sdf.fill((COLOR_UP_3));
                    let kw = sz * 1.3;
                    let kx = left + px + self.active * (tw - kw - 2.0 * px);
                    sdf.rect(kx, cy - sz + px, kw, th - 2.0 * px);
                    sdf.fill(mix((COLOR_PALE), (COLOR_HILITE), self.hover));
                    sdf.rect(kx + kw - px, cy - sz + px, px, th - 2.0 * px);
                    sdf.fill((COLOR_DOWN_5));
                    return sdf.result
                }
            }
            draw_text: {
                text_style: <H2_TEXT_BOLD> {},
                color: (COLOR_UP_5)
            }
        }
    }

    LabelToggle = <FishToggle> {
        width: Fit, height: Fit
        margin: <SPACING_0> {}
        padding: <SPACING_0> {}
        checkbox = { width: Fit, text: "Freeze", padding: {right: (SSPACING_2)} }
    }

    InstrumentDropdown = <ElementBox> {
        align: {y: 0.5}
        padding: <SPACING_0> {},
        flow: Right
        dropdown = <FishDropDown> {
            margin: {left: (SSPACING_1), right: (SSPACING_1)}
        }
    }

    PxButton = <View> {
        width: 28, height: 28
        flow: Overlay
        align: {x: 0.5, y: 0.5}
        button = <FishButton> { width: Fill, height: Fill, padding: 0, margin: 0, text: "" }
        icon = <PixelIcon> { width: 20, height: 20 }
    }

    GrainSlider = <InstrumentSlider> {
        width: Fill
        margin: <SPACING_0> {}
        padding: <SPACING_0> {}
        slider = { draw_bg: {line_color: (COLOR_HOT)} min: 0.0 max: 1.0 }
    }

    GrainBipolarSlider = <InstrumentBipolarSlider> {
        width: Fill
        margin: <SPACING_0> {}
        padding: <SPACING_0> {}
        slider = { draw_bg: {line_color: (COLOR_HOT)} min: -1.0 max: 1.0 }
    }

    GrainLabel = <Label> {
        width: Fill, height: Fit
        margin: {top: (SSPACING_2), bottom: 0, left: (SSPACING_1)}
        draw_text: { text_style: <H2_TEXT_BOLD> {}, color: (COLOR_HOT) }
        text: "Grain"
    }

    Title = <Label> {
        width: Fit, height: Fit
        margin: {left: (SSPACING_2), top: (SSPACING_1)}
        draw_text: { text_style: <PIXEL_FONT>{ font_size: 9.0 }, color: (COLOR_PALE) }
        text: "Manguo Grain"
    }

    pub App = {{App}} {
        ui: <Root>{
            main_window = <Window>{
                window: {inner_size: vec2(640, 640), title: "Manguo Grain"},
                pass: {clear_color: (COLOR_BG)}
                show_bg: true
                draw_bg: { color: (COLOR_BG) }

                body = <View>{
                    flow: Down
                    width: Fill, height: Fill
                    padding: <SPACING_2> {}
                    spacing: (SSPACING_1)

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {x: 0.0, y: 0.5}
                        <Title> {}
                        <FillerH> {}
                        meter = <Label> {
                            width: Fit, height: Fit
                            margin: {right: (SSPACING_2)}
                            draw_text: { text_style: <H2_TEXT_BOLD> {}, color: (COLOR_UP_5) }
                            text: ""
                        }
                    }

                    <FishPanel> {
                        height: Fill
                        flow: Down
                        padding: <SPACING_0> {}
                        draw_bg: {
                            color_1: (COLOR_DOWN_3),
                            color_2: (COLOR_DOWN_4)
                            bevel_light: (COLOR_DOWN_5)
                            bevel_dark: (COLOR_UP_3)
                        }

                        // transport row
                        <View> {
                            height: Fit, width: Fill
                            flow: Right,
                            align: {x: 0.0, y: 0.5}
                            spacing: (SSPACING_4),
                            padding: {top: (SSPACING_1), right: (SSPACING_3), bottom: (SSPACING_0), left: (SSPACING_0)}

                            playpause = <LabelToggle> { margin: {left: (SSPACING_2)} checkbox = { text: "Run" } }
                            freeze = <LabelToggle> { checkbox = { text: "Freeze" } }

                            delaytime = <InstrumentSlider> {
                                width: Fill
                                slider = {
                                    draw_bg: {line_color: (COLOR_DEFAULT)}
                                    min: 0.1 max: 2.0 default: 0.5 precision: 2 text: "Time s"
                                }
                            }
                            repeats = <InstrumentSlider> {
                                width: Fill
                                slider = { draw_bg: {line_color: (COLOR_HOT)} min: 0.0 max: 1.0 default: 0.6 text: "Repeats" }
                            }
                        }

                        <View> {
                            width: Fill, height: Fit
                            flow: Right
                            spacing: (SSPACING_1)
                            padding: {bottom: (SSPACING_2)}

                            <View> {
                                width: 276, height: Fit
                                flow: Down
                                sequencer = <Sequencer> {width: Fill, height: 256, margin: {top: (SSPACING_1), bottom: (SSPACING_1), left: (SSPACING_2), right: (SSPACING_2)}}

                                <View> {
                                    height: Fit, width: Fill
                                    flow: Down
                                    padding: {left: (SSPACING_2), right: (SSPACING_2)}

                                    <View> {
                                        height: Fit, width: Fill
                                        flow: Right, spacing: (SSPACING_1)
                                        align: {x: 0.0, y: 0.5}
                                        delaymode = <InstrumentDropdown> {
                                            height: Fit, width: Fit
                                            dropdown = { labels: ["Digital", "Analog", "Tape"] values: [Digital, Analog, Tape] }
                                        }
                                        scaletype = <InstrumentDropdown> {
                                            height: Fit, width: Fit
                                            dropdown = { labels: ["Minor", "Major", "Dorian", "Pentatonic"] values: [Minor, Major, Dorian, Pentatonic] }
                                        }
                                        <FillerH> {}
                                        clear_grid = <IconButton> {draw_icon: {svg_file: (ICO_SEQ_SWEEP)} icon_walk: {width: 15.0, height: Fit}}
                                        grid_up = <IconButton> {draw_icon: {svg_file: (ICO_UP)} icon_walk: {width: 10.0, height: Fit}}
                                        grid_down = <IconButton> {draw_icon: {svg_file: (ICO_DOWN)} icon_walk: {width: 10.0, height: Fit}}
                                    }

                                    <View> {
                                        height: Fit, width: Fill
                                        flow: Right, spacing: (SSPACING_1)
                                        padding: {top: (SSPACING_2), bottom: (SSPACING_0)}
                                        align: {x: 0.0, y: 0.5}
                                        <PixelIcon> { source: (PX_CELL), width: 18, height: 18, margin: {right: 2} }
                                        life = <LabelToggle> { checkbox = { text: "Life" } }
                                        stamp = <LabelToggle> { margin: {left: (SSPACING_2)} checkbox = { text: "Stamp" } }
                                        <FillerH> {}
                                        lifeseed = <PxButton> { icon = { source: (PX_SEED) } }
                                    }

                                    <GrainLabel> { text: "Input" }
                                    mixinput = <GrainSlider> { slider = { min: 0.0 max: 4.0 default: 1.0 precision: 2 text: "Input" } }
                                    voicesens = <GrainSlider> { slider = { default: 0.5 text: "Sensitivity" } }
                                    gaterelease = <GrainSlider> { slider = { min: 0.03 max: 2.0 default: 0.3 precision: 2 text: "Release s" } }
                                    mixdrywet = <GrainSlider> { slider = { default: 0.5 text: "Dry / Wet" } }
                                }
                            }

                            <View> {
                                height: Fit, width: Fill
                                flow: Down
                                padding: {left: (SSPACING_1), right: (SSPACING_2)}
                                <GrainLabel> { text: "Grain" }
                                grainsize = <GrainSlider> { slider = { min: 1.0 max: 4.0 default: 2.0 precision: 1 text: "Size" } }
                                grainpitch = <GrainSlider> { slider = { default: 1.0 text: "Pitch" } }
                                grainreverse = <GrainSlider> { slider = { default: 0.0 text: "Reverse" } }
                                <GrainLabel> { text: "Motion" }
                                grainspray = <GrainSlider> { slider = { default: 0.0 text: "Spray" } }
                                graindensity = <GrainSlider> { slider = { default: 1.0 text: "Density" } }
                                grainwidth = <GrainSlider> { slider = { default: 0.5 text: "Width" } }
                                <GrainLabel> { text: "Return" }
                                returntone = <GrainBipolarSlider> { slider = { default: 0.0 text: "Tone LP/HP" } }
                                returnduck = <GrainSlider> { slider = { default: 0.0 text: "Duck" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// What the editor shares with the host side.
pub struct PluginUi {
    pub params: Arc<GrainParams>,
    pub shared: Arc<GridShared>,
    pub gui: GuiContext,
}

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    pub plug: Option<Rc<PluginUi>>,
    #[rust]
    sync_timer: Timer,
    #[rust]
    last_steps: [u32; W],
    #[rust]
    last_step: usize,
    #[rust]
    last_meter: String,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        crate::makepad_widgets::live_design(cx);
        crate::sequencer::live_design(cx);
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        self.sync_timer = cx.start_interval(1.0 / 30.0);
        self.sync_from_params(cx, true);
    }

    fn handle_timer(&mut self, cx: &mut Cx, e: &TimerEvent) {
        if self.sync_timer.is_timer(e).is_some() {
            self.sync_from_params(cx, false);
        }
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        let Some(plug) = self.plug.clone() else { return };
        let p = &plug.params;
        let setter = plug.gui.param_setter();
        let ui = self.ui.clone();

        // grid: clicks, clear, shifts
        let sequencer = ui.sequencer(id!(sequencer));
        if ui.button_set(ids!(clear_grid)).clicked(actions) {
            sequencer.clear_grid(cx);
        }
        if ui.button_set(ids!(grid_down)).clicked(actions) {
            sequencer.grid_down(cx);
        }
        if ui.button_set(ids!(grid_up)).clicked(actions) {
            sequencer.grid_up(cx);
        }
        if actions.find_widget_action(sequencer.widget_uid()).is_some() {
            if let Some(inner) = sequencer.borrow() {
                let steps = inner.get_steps(cx);
                let mut arr = [0u32; W];
                for (i, s) in steps.iter().take(W).enumerate() {
                    arr[i] = *s;
                }
                plug.shared.set_steps(&arr);
                self.last_steps = arr;
            }
        }
        if ui.button_set(ids!(lifeseed.button)).clicked(actions) {
            plug.shared.seed_request.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // toggles
        let toggles: [(&[&[LiveId]], &BoolParam); 4] = [
            (ids!(playpause.checkbox), &p.running),
            (ids!(freeze.checkbox), &p.freeze),
            (ids!(life.checkbox), &p.life),
            (ids!(stamp.checkbox), &p.auto_stamp),
        ];
        for (id, param) in toggles {
            if let Some(on) = ui.check_box_set(id).changed(actions) {
                setter.begin_set_parameter(param);
                setter.set_parameter(param, on);
                setter.end_set_parameter(param);
            }
        }

        // dropdowns
        if let Some(i) = ui.drop_down(id!(delaymode.dropdown)).selected(actions) {
            let v = match i {
                1 => crate::ModeParam::Analog,
                2 => crate::ModeParam::Tape,
                _ => crate::ModeParam::Digital,
            };
            setter.begin_set_parameter(&p.mode);
            setter.set_parameter(&p.mode, v);
            setter.end_set_parameter(&p.mode);
        }
        if let Some(i) = ui.drop_down(id!(scaletype.dropdown)).selected(actions) {
            let v = match i {
                1 => crate::ScaleParam::Major,
                2 => crate::ScaleParam::Dorian,
                3 => crate::ScaleParam::Pentatonic,
                _ => crate::ScaleParam::Minor,
            };
            setter.begin_set_parameter(&p.scale);
            setter.set_parameter(&p.scale, v);
            setter.end_set_parameter(&p.scale);
        }

        // sliders: begin/end gestures map to the host's automation gestures
        let sliders: [(&[LiveId], &FloatParam); 13] = [
            (id!(delaytime.slider), &p.time),
            (id!(repeats.slider), &p.repeats),
            (id!(grainsize.slider), &p.size),
            (id!(grainpitch.slider), &p.pitch),
            (id!(grainreverse.slider), &p.reverse),
            (id!(grainspray.slider), &p.spray),
            (id!(graindensity.slider), &p.density),
            (id!(grainwidth.slider), &p.width),
            (id!(returntone.slider), &p.tone),
            (id!(returnduck.slider), &p.duck),
            (id!(mixinput.slider), &p.input),
            (id!(voicesens.slider), &p.sensitivity),
            (id!(gaterelease.slider), &p.gate_release),
        ];
        for (id, param) in sliders {
            let slider = ui.slider(id);
            if let Some(item) = actions.find_widget_action(slider.widget_uid()) {
                match item.cast() {
                    SliderAction::StartSlide => setter.begin_set_parameter(param),
                    SliderAction::Slide(v) => setter.set_parameter(param, v as f32),
                    SliderAction::EndSlide => setter.end_set_parameter(param),
                    SliderAction::TextSlide(v) => {
                        setter.begin_set_parameter(param);
                        setter.set_parameter(param, v as f32);
                        setter.end_set_parameter(param);
                    }
                    _ => {}
                }
            }
        }
        if let Some(item) = actions.find_widget_action(ui.slider(id!(mixdrywet.slider)).widget_uid()) {
            match item.cast() {
                SliderAction::StartSlide => setter.begin_set_parameter(&p.mix),
                SliderAction::Slide(v) => setter.set_parameter(&p.mix, v as f32),
                SliderAction::EndSlide => setter.end_set_parameter(&p.mix),
                SliderAction::TextSlide(v) => {
                    setter.begin_set_parameter(&p.mix);
                    setter.set_parameter(&p.mix, v as f32);
                    setter.end_set_parameter(&p.mix);
                }
                _ => {}
            }
        }
    }
}

impl App {
    /// Push host-side parameter values and the published grid into the widgets.
    fn sync_from_params(&mut self, cx: &mut Cx, force: bool) {
        let Some(plug) = self.plug.clone() else { return };
        let p = &plug.params;
        let ui = self.ui.clone();

        let set_slider = |cx: &mut Cx, id: &[LiveId], v: f32| {
            let s = ui.slider(id);
            if force || s.value().map(|cur| (cur - v as f64).abs() > 1e-4).unwrap_or(true) {
                s.set_value(cx, v as f64);
            }
        };
        set_slider(cx, id!(delaytime.slider), p.time.value());
        set_slider(cx, id!(repeats.slider), p.repeats.value());
        set_slider(cx, id!(grainsize.slider), p.size.value());
        set_slider(cx, id!(grainpitch.slider), p.pitch.value());
        set_slider(cx, id!(grainreverse.slider), p.reverse.value());
        set_slider(cx, id!(grainspray.slider), p.spray.value());
        set_slider(cx, id!(graindensity.slider), p.density.value());
        set_slider(cx, id!(grainwidth.slider), p.width.value());
        set_slider(cx, id!(returntone.slider), p.tone.value());
        set_slider(cx, id!(returnduck.slider), p.duck.value());
        set_slider(cx, id!(mixinput.slider), p.input.value());
        set_slider(cx, id!(voicesens.slider), p.sensitivity.value());
        set_slider(cx, id!(gaterelease.slider), p.gate_release.value());
        set_slider(cx, id!(mixdrywet.slider), p.mix.value());

        let set_toggle = |cx: &mut Cx, id: &[&[LiveId]], on: bool| {
            ui.check_box_set(id).set_active(cx, on);
        };
        set_toggle(cx, ids!(playpause.checkbox), p.running.value());
        set_toggle(cx, ids!(freeze.checkbox), p.freeze.value());
        set_toggle(cx, ids!(life.checkbox), p.life.value());
        set_toggle(cx, ids!(stamp.checkbox), p.auto_stamp.value());

        let mode = match p.mode.value() {
            crate::ModeParam::Digital => 0,
            crate::ModeParam::Analog => 1,
            crate::ModeParam::Tape => 2,
        };
        let dd = ui.drop_down(id!(delaymode.dropdown));
        if force || dd.selected_item() != mode {
            dd.set_selected_item(cx, mode);
        }
        let scale = match p.scale.value() {
            crate::ScaleParam::Minor => 0,
            crate::ScaleParam::Major => 1,
            crate::ScaleParam::Dorian => 2,
            crate::ScaleParam::Pentatonic => 3,
        };
        let dd = ui.drop_down(id!(scaletype.dropdown));
        if force || dd.selected_item() != scale {
            dd.set_selected_item(cx, scale);
        }

        // grid published by the engine, playhead
        let steps = plug.shared.steps();
        if force || steps != self.last_steps {
            self.last_steps = steps;
            ui.sequencer_set(ids!(sequencer)).set_steps(cx, &steps);
        }
        let step = plug.shared.current_slot.load(std::sync::atomic::Ordering::Relaxed) as usize;
        if force || step != self.last_step {
            self.last_step = step;
            ui.sequencer_set(ids!(sequencer)).set_step(cx, step);
        }
        let present = plug.shared.present.load(std::sync::atomic::Ordering::Relaxed);
        let meter = format!(
            "in {:>4.2}  grains {:>4.2}  pop {:>3}{}",
            plug.shared.input_level(),
            plug.shared.take_grain_level(),
            plug.shared.population.load(std::sync::atomic::Ordering::Relaxed),
            if present { "  *" } else { "" }
        );
        if meter != self.last_meter {
            self.last_meter = meter.clone();
            ui.label(id!(meter)).set_text(cx, &meter);
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
