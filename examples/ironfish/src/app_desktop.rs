use crate::makepad_widgets::*;
 
live_design! {
    use link::widgets::*
    use link::theme::*
    use link::shaders::*

    use makepad_example_ironfish::sequencer::Sequencer;
    use makepad_example_ironfish::life_fractal::LifeFractal;
    use makepad_audio_widgets::display_audio::DisplayAudio;
    use makepad_audio_widgets::piano::Piano;

    FONT_SIZE_H2 = 6.5

    HEIGHT_AUDIOVIZ = 150

    // 8-bit look: one "virtual pixel" in layout points.
    PX = 3.0

    SSPACING_0 = 0.0
    SSPACING_1 = 4.0
    SSPACING_2 = (SSPACING_1 * 2)
    SSPACING_3 = (SSPACING_1 * 3)
    SSPACING_4 = (SSPACING_1 * 4)

    SPACING_0 = {top: (SSPACING_0), right: (SSPACING_0), bottom: (SSPACING_0), left: (SSPACING_0)}
    SPACING_1 = {top: (SSPACING_1), right: (SSPACING_1), bottom: (SSPACING_1), left: (SSPACING_1)}
    SPACING_2 = {top: (SSPACING_2), right: (SSPACING_2), bottom: (SSPACING_2), left: (SSPACING_2)}
    SPACING_3 = {top: (SSPACING_3), right: (SSPACING_3), bottom: (SSPACING_3), left: (SSPACING_3)}
    SPACING_4 = {top: (SSPACING_4), right: (SSPACING_4), bottom: (SSPACING_4), left: (SSPACING_4)}
    
    // Pixel font (Press Start 2P, SIL OFL).
    PIXEL_FONT = <THEME_FONT_BOLD>{
        font_family: {
            latin = font("crate://self/resources/PressStart2P-Regular.ttf", 0.0, 0.0),
        },
        line_spacing: 1.4
    }

    H2_TEXT_BOLD = <PIXEL_FONT>{
        font_size: (FONT_SIZE_H2),
    }
    
    H2_TEXT_REGULAR = <PIXEL_FONT>{
        font_size: (FONT_SIZE_H2),
    }

    // Pink 8-bit palette
    COLOR_BG = #x1A0A14
    COLOR_PANEL = #x2B1020
    COLOR_PANEL_2 = #x3A1530
    COLOR_HOT = #xFF3FA4
    COLOR_MAGENTA = #xE0218A
    COLOR_PALE = #xFFB3DA
    COLOR_HILITE = #xFFE1F0

    COLOR_DOWN_FULL = #x0A0208

    COLOR_DOWN_0 = #x0A020800
    COLOR_DOWN_1 = #x0A020822
    COLOR_DOWN_2 = #x0A020844
    COLOR_DOWN_3 = #x0A020866
    COLOR_DOWN_4 = #x0A020899
    COLOR_DOWN_5 = #x0A0208CC
    COLOR_DOWN_6 = #x0A0208EE

    COLOR_UP_0 = #xFFB3DA00
    COLOR_UP_1 = #xFFB3DA12
    COLOR_UP_2 = #xFFB3DA1E
    COLOR_UP_3 = #xFFB3DA30
    COLOR_UP_4 = #xFFB3DA55
    COLOR_UP_5 = #xFFB3DA99
    COLOR_UP_6 = #xFFB3DAE6
    COLOR_UP_FULL = #xFFE1F0FF

    COLOR_ALERT = (COLOR_HOT)
    COLOR_OSC = #xFF7AC8
    COLOR_ENV = #xFFB3DA
    COLOR_FILTER = #xFF3FA4
    COLOR_FX = #xD98CFF
    COLOR_DEFAULT = (COLOR_UP_6)

    COLOR_VIZ_1 = (COLOR_DOWN_2)
    COLOR_VIZ_2 = (COLOR_DOWN_6)
    COLOR_DIVIDER = (COLOR_DOWN_5)

    // Pixel-art assets
    PX_MIC = dep("crate://self/resources/px/mic.png")
    PX_CELL = dep("crate://self/resources/px/cell.png")
    PX_SPIRAL = dep("crate://self/resources/px/spiral.png")
    PX_CAT = dep("crate://self/resources/px/cat.png")
    PX_SKULL = dep("crate://self/resources/px/skull.png")
    PX_SEED = dep("crate://self/resources/px/seed.png")
    PX_BG = dep("crate://self/resources/px/bg_hearts.png")
    PHOTO_CAT = dep("crate://self/resources/voice_cat.jpg")
    PHOTO_ROY = dep("crate://self/resources/voice_roy.jpg")

    ICO_ARP = dep("crate://self/resources/icons/Icon_Arp.svg")
    ICO_BROWSE = dep("crate://self/resources/icons/Icon_Browse.svg")
    ICO_DOWN = dep("crate://self/resources/icons/Icon_Down.svg")
    ICO_FAV = dep("crate://self/resources/icons/Icon_Favorite.svg")
    ICO_FILTER_BP = dep("crate://self/resources/icons/Icon_Filters_BP.svg")
    ICO_FILTER_BR = dep("crate://self/resources/icons/Icon_Filters_BR.svg")
    ICO_FILTER_HP = dep("crate://self/resources/icons/Icon_Filters_HP.svg")
    ICO_LIVEPLAY = dep("crate://self/resources/icons/Icon_LivePlaying.svg")
    ICO_NEXT = dep("crate://self/resources/icons/Icon_Next.svg")
    ICO_OSC_HARMONIC = dep("crate://self/resources/icons/Icon_OSC_Harmonic.svg")
    ICO_OSC_SAW = dep("crate://self/resources/icons/Icon_OSC_Saw.svg")
    ICO_OSC_SINE = dep("crate://self/resources/icons/Icon_OSC_Sine.svg")
    ICO_OSC_SUPERSAW = dep("crate://self/resources/icons/Icon_OSC_Supersaw.svg")
    ICO_OSC_TRI = dep("crate://self/resources/icons/Icon_OSC_Tri.svg")
    ICO_PANIC = dep("crate://self/resources/icons/Icon_Panic.svg")
    ICO_PLAT_MOBILE = dep("crate://self/resources/icons/Icon_Platform_Mobile.svg")
    ICO_PLAT_DESKTOP = dep("crate://self/resources/icons/Icon_Platform_Desktop.svg")
    ICO_PLAY = dep("crate://self/resources/icons/Icon_Play.svg")
    ICO_PRESET = dep("crate://self/resources/icons/Icon_Presets.svg")
    ICO_PREV = dep("crate://self/resources/icons/Icon_Prev.svg")
    ICO_REDO = dep("crate://self/resources/icons/Icon_Redo.svg")
    ICO_SAVE = dep("crate://self/resources/icons/Icon_Save.svg")
    ICO_SEARCH = dep("crate://self/resources/icons/Icon_Search.svg")
    ICO_SEQ_SWEEP = dep("crate://self/resources/icons/Icon_Seq_Sweep.svg")
    ICO_SEQ = dep("crate://self/resources/icons/Icon_Seq.svg")
    ICO_SETTINGS = dep("crate://self/resources/icons/Icon_Settings.svg")
    ICO_SHARE = dep("crate://self/resources/icons/Icon_Share.svg")
    ICO_UNDO = dep("crate://self/resources/icons/Icon_Undo.svg")
    ICO_UP = dep("crate://self/resources/icons/Icon_Up.svg")

    // 8-BIT HELPERS

    // Image resampled on a coarse virtual-pixel grid (nearest neighbour),
    // so both pixel-art assets and photos read as chunky pixels.
    PixelImage = <Image> {
        fit: Smallest
        draw_bg: {
            instance grid: 48.0
            instance tint: 1.0
            fn get_color(self) -> vec4 {
                let cell = floor(self.pos * self.grid);
                let uv = (cell + 0.5) / self.grid;
                let c = sample2d(self.image, uv * self.image_scale + self.image_pan);
                // quantize to 5 levels per channel and tint pink
                let q = floor(c.xyz * 5.0 + 0.5) / 5.0;
                let pinked = mix(q, q * vec3(1.15, 0.85, 1.05), self.tint * 0.6);
                return vec4(pinked, c.w);
            }
        }
    }

    // Small generated pixel-art icon (crisp, no tint).
    PixelIcon = <PixelImage> {
        width: 24, height: 24
        draw_bg: { grid: 24.0, tint: 0.0 }
    }

    // The logo: 芒果 rendered to a texture, then rebuilt from big bevelled squares.
    PixelLogoInner = <CachedRoundedView> {
        width: 440,
        height: 400,
        margin: {top: -94}
        align: {x: 0.5, y: 0.5}
        draw_bg: {
            instance cellsize: 6.0
            instance glow: 0.0
            fn pixel(self) -> vec4 {
                let cs = self.cellsize;
                let p = self.pos * self.rect_size;
                let cell = floor(p / cs);
                let uv = (cell + 0.5) * cs / self.rect_size;
                let c = sample2d_rt(self.image, uv * self.scale + self.shift);
                if c.w < 0.5 {
                    return vec4(0.0);
                }
                let f = p - cell * cs;
                let gap = 1.0;
                if (f.x < gap || f.y < gap) {
                    return vec4(0.0);
                }
                let light = step(f.x, gap + 1.0) + step(f.y, gap + 1.0);
                let dark = step(cs - 1.0, f.x) + step(cs - 1.0, f.y);
                let base = mix((COLOR_HOT), (COLOR_HILITE), self.glow * 0.6);
                let col = base;
                if dark > 0.0 {
                    col = (COLOR_MAGENTA) * vec4(0.7, 0.7, 0.7, 1.0);
                }
                else if light > 0.0 {
                    col = (COLOR_HILITE);
                }
                return Pal::premul(vec4(col.xyz, 1.0));
            }
        }
        glyphs = <Label> {
            draw_text: {
                text_style: <THEME_FONT_BOLD> {font_size: 150},
                color: #fff
            }
            text: "芒果"
        }
    }

    PixelLogo = <View> {
        width: 440,
        height: 96,
        clip_x: true, clip_y: true,
        logo_inner = <PixelLogoInner> {}
    }

    // Half-size variant for the life window title
    PixelLogoSmall = <View> {
        width: 220,
        height: 48,
        clip_x: true, clip_y: true,
        logo_inner = <PixelLogoInner> {
            width: 220, height: 200
            margin: {top: -47}
            draw_bg: { cellsize: 3.0 }
            glyphs = { draw_text: { text_style: <THEME_FONT_BOLD> {font_size: 75} } }
        }
    }


    // Square-cornered RoundedView (8-bit: no rounding anywhere)
    SquareView = <RoundedView> {
        draw_bg: { border_radius: 0.0 }
    }

    // HELPERS
    FillerH = <View> {
        width: Fill
    }

    FillerV = <View> {
        height: Fill
    }

    Divider = <View> {
        width: Fill,
        height: Fit,
        margin: {top: (SSPACING_3), right: 0, bottom: (SSPACING_3), left: (SSPACING_0)}
        flow: Down
            <SquareView> {
            width: Fill,
            height: 1.0
            draw_bg: {color: (COLOR_DIVIDER)}
        }
        <SquareView> {
            width: Fill,
            height: 1.0
            draw_bg: {color: (COLOR_UP_4)}
        }
    }


    // WIDGETS
    ElementBox = <View> {
        draw_bg: {color: (COLOR_DOWN_0)}
        width: Fill,
        height: Fit
        flow: Down,
        padding: <SPACING_1> {}
        spacing: (SSPACING_1)
    }

    FishPanelContainer = <View> {
        flow: Down
        width: Fill,
        height: Fit
    }

    SubheaderContainer = <SquareView> {
        draw_bg: {color: (COLOR_UP_2)}
        width: Fill,
        height: Fit,
        align: { x: 0.0, y: 0.5 }
        margin: {bottom: (SSPACING_2), top: (SSPACING_2)}
        padding: {top: (SSPACING_0), right: (SSPACING_1), bottom: (SSPACING_0), left: (SSPACING_1)}
    }

    FishSubTitle = <View> {
        width: Fit,
        height: Fit,
        margin: {top: 1}
        padding: {top: (SSPACING_2), right: (SSPACING_1), bottom: (SSPACING_2), left: (SSPACING_1)}

        label = <Label> {
            draw_text: {
                text_style: <H2_TEXT_BOLD> {},
                color: (COLOR_UP_5)
            }
            text: "replace me!"
        }
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

            // banded (posterized) vertical gradient
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
                // 8-bit bevel: light on top/left, dark on bottom/right
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

    FishPanelScrollY = <FishPanel> {
        height: Fill
        scroll_bars: <ScrollBars> {show_scroll_x: false, show_scroll_y: true}
    }

    FishDropDown = <DropDownFlat> {
        width: Fit
        // padding: {top: (SSPACING_2), right: (SSPACING_4), bottom: (SSPACING_2), left: (SSPACING_2)}

    }

    IconButton = <Button> {
        draw_icon: {
            svg_file: (ICO_SAVE),
            fn get_color(self) -> vec4 {
                return mix(
                    mix(
                        (COLOR_UP_5),
                        (COLOR_UP_6),
                        self.hover
                    ),
                    (COLOR_UP_4),
                    self.down
                )
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

    TextButton = <Button> {
        align: {x: 0.5, y: 0.5}
        padding: <SPACING_0> {}
        margin: {left: 2.5, right: 2.5}

        draw_text: {
            text_style: <H2_TEXT_BOLD> {}
            fn get_color(self) -> vec4 {
                return mix(
                    mix(
                        (COLOR_ALERT),
                        (COLOR_UP_FULL),
                        self.hover
                    ),
                    (COLOR_DOWN_FULL),
                    self.down
                )
            }
        }

        draw_bg: {
            instance hover: 0.0
            instance down: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                return sdf.result
            }
        }

    }

    FishButton = <Button> {

        align: {x: 0.5, y: 0.5}
        padding: <SPACING_2> {}

        draw_text: {
            text_style: <H2_TEXT_BOLD> {}
            fn get_color(self) -> vec4 {
                return mix(
                    mix(
                        (COLOR_UP_5),
                        (COLOR_UP_6),
                        self.hover
                    ),
                    (COLOR_UP_5),
                    self.down
                )
            }
        }

        draw_bg: {
            instance hover: 0.0
            instance down: 0.0

            fn pixel(self) -> vec4 {
                // NES-style button: flat fill, 2px bevel; bevel flips when pressed
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
            // cursor_margin_bottom: (SSPACING_1),
            // cursor_margin_top: (SSPACING_1),
            // select_pad_edges: (SSPACING_1),
            // cursor_size: (SSPACING_1),
            empty_text: "0",
            is_numeric_only: true,
            draw_bg: {
                color: (COLOR_DOWN_0)
            },
        }
        draw_bg: {
            instance line_color: (COLOR_HOT)
            instance bipolar: 0.0
            fn pixel(self) -> vec4 {
                // 8-bit slider: square trough, blocky fill bar, square nub
                let px = (PX);
                let p = floor(self.pos * self.rect_size / px) * px;
                let top = 21.0;
                let w = self.rect_size.x;
                let h = self.rect_size.y;
                let sdf = Sdf2d::viewport(p);

                // trough
                sdf.rect(px, top, w - 2.0 * px, h - top - px);
                sdf.fill_keep(mix((COLOR_DOWN_4), (COLOR_DOWN_6), self.drag));
                // bevel: dark top edge, light bottom edge (sunken)
                sdf.rect(px, top, w - 2.0 * px, px);
                sdf.fill((COLOR_DOWN_6));
                sdf.rect(px, h - 2.0 * px, w - 2.0 * px, px);
                sdf.fill((COLOR_UP_3));

                // value bar, from origin (left or centre when bipolar) to nub
                let inner = w - 4.0 * px;
                let origin = mix(2.0 * px, w * 0.5, self.bipolar);
                let nub_x = 2.0 * px + self.slide_pos * (inner - 3.0 * px);
                let bar_x0 = min(origin, nub_x);
                let bar_x1 = max(origin, nub_x + 3.0 * px);
                sdf.rect(bar_x0, top + 2.0 * px, bar_x1 - bar_x0, h - top - 4.0 * px);
                sdf.fill(mix(self.line_color * 0.8, self.line_color, self.hover + self.drag));

                // nub: 3px-wide raised block
                sdf.rect(nub_x, top + px, 3.0 * px, h - top - 2.0 * px);
                sdf.fill_keep(mix((COLOR_PALE), (COLOR_HILITE), self.hover));
                sdf.rect(nub_x + 2.0 * px, top + px, px, h - top - 2.0 * px);
                sdf.fill((COLOR_MAGENTA));

                return sdf.result
            }
        }
    }

    InstrumentSlider = <ElementBox> {
        slider = <FishSlider> {
            draw_bg: {bipolar: 0.0}
        }
    }

    InstrumentBipolarSlider = <ElementBox> {
        slider = <FishSlider> {
            draw_bg: {bipolar: 1.0}
        }
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
                instance border_width: 1.0
                instance border_color: #x06
                instance border_color2: #xFFFFFF0A
                size: 8.5;
                fn pixel(self) -> vec4 {
                    // 8-bit toggle: square track, square knob that slides in one step
                    let px = (PX);
                    let sdf = Sdf2d::viewport(floor(self.pos * self.rect_size / px) * px)
                    let sz = self.size;
                    let left = px;
                    let cy = self.rect_size.y * 0.5;
                    let tw = sz * 3.0;
                    let th = sz * 2.0;
                    sdf.rect(left, cy - sz, tw, th);
                    sdf.fill(mix((COLOR_DOWN_5), (COLOR_MAGENTA), self.active));
                    // bevel
                    sdf.rect(left, cy - sz, tw, px);
                    sdf.fill((COLOR_DOWN_6));
                    sdf.rect(left, cy + sz - px, tw, px);
                    sdf.fill((COLOR_UP_3));
                    // knob
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

    InstrumentDropdown = <ElementBox> {
        align: {y: 0.5}
        padding: <SPACING_0> {},
        flow: Right
        dropdown = <FishDropDown> {
            margin: {left: (SSPACING_1), right: (SSPACING_1)}
        }
    }

    GraphPaper = <SquareView> {
        width: Fill,
        height: 120
        draw_bg: {
            color: #x44,
            instance color2: #x0,

            instance attack: 0.05
            instance hold: 0.0
            instance decay: 0.2
            instance sustain: 0.5
            instance release: 0.2

            fn pixel(self) -> vec4 {
                let px = (PX);
                let pos = floor(self.pos * self.rect_size / px) * px;
                let sdf = Sdf2d::viewport(pos);
                let base_color = (COLOR_DOWN_4);
                let darker = (COLOR_MAGENTA) * 0.35;
                // 8-bit graph paper: a grid line every 4 virtual pixels
                let gx = step(mod(pos.x, px * 4.0), px * 0.5);
                let gy = step(mod(pos.y, px * 4.0), px * 0.5);
                sdf.clear(mix(base_color, darker, clamp(gx + gy, 0.0, 1.0)));
                let pad_b = 8
                let pad_s = 8
                let width = self.rect_size.x - 2 * pad_s
                let height = self.rect_size.y - 2 * pad_b
                let total = self.attack + self.decay + self.release + 0.5 + self.hold
                let sustain = self.rect_size.y - pad_b - height * self.sustain;
                sdf.pos = self.pos * self.rect_size;
                sdf.move_to(pad_s, self.rect_size.y - pad_b)
                sdf.line_to(pad_s + width * (self.attack / total), pad_b)
                sdf.line_to(pad_s + width * ((self.attack + self.hold) / total), pad_b)
                sdf.line_to(pad_s + width * ((self.attack + self.decay + self.hold) / total), sustain)
                sdf.line_to(pad_s + width * (1.0 - self.release / total), sustain)
                sdf.line_to(pad_s + width, self.rect_size.y - pad_b)
                sdf.stroke_keep((COLOR_MAGENTA), px * 2.0);
                sdf.stroke((COLOR_HILITE), px);
                return sdf.result
            }
        }
    }

    FishTitle = <SquareView> {
        width: Fit,
        height: Fit,
        margin: {bottom: (SSPACING_1)}
        padding: <SPACING_2> {}
        label = <Label> {
            margin: {top: 1}
            draw_text: {
                text_style: <H2_TEXT_BOLD> {},
                color: (COLOR_DOWN_6)
            }
            text: "replace me!"
        }
    }

    FishHeader = <SquareView> {
        flow: Right
        height: Fit,
        width: Fill,
        align: { x: 0.0, y: 0.5 }
        margin: {bottom: (SSPACING_2)}
        title = <FishTitle> {
            height: Fit,
            width: Fill,
            margin: <SPACING_0> {}
            padding: <SPACING_2> {}
        }
        menu = <View> {
            flow: Right
            height: Fit,
            width: Fit
        }
    }

    CheckboxTextual = <CheckBox> {
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size)

                return sdf.result
            }
        }

        draw_text: {
            text_style: <H2_TEXT_REGULAR> {},
            fn get_color(self) -> vec4 {
                return mix(
                    (COLOR_UP_4),
                    (COLOR_UP_6),
                    self.active
                )
            }
        }

        label_walk: {margin: {left: 0.0}}

    }

    PlayPause = <FishToggle> {
        width: Fit,
        height: Fit,
        margin: <SPACING_3> {}
        align: {x: 0.5, y: 0.5}
        checkbox = {
            width: 30,
            height: 30,
            margin: {right: -20}
            text: ""
            animator: {
                hover = {
                    default: off
                    off = {
                        from: {all: Forward {duration: 0.1}}
                        apply: {
                            draw_bg: {hover: 0.0}
                        }
                    }
                    on = {
                        from: {all: Snap}
                        apply: {
                            draw_bg: {hover: 1.0}
                        }
                    }
                }
                focus = {
                    default: off
                    off = {
                        from: {all: Forward {duration: 0.0}}
                        apply: {
                            draw_bg: {focus: 0.0}
                        }
                    }
                    on = {
                        from: {all: Snap}
                        apply: {
                            draw_bg: {focus: 1.0}
                        }
                    }
                }
                active = {
                    default: off
                    off = {
                        from: {all: Forward {duration: 0.0}}
                        apply: {draw_bg: {active: 0.0}}
                    }
                    on = {
                        cursor: Arrow,
                        from: {all: Forward {duration: 0.0}}
                        apply: {draw_bg: {active: 1.0}}
                    }
                }
            }
            draw_bg: {
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                    let sz = self.rect_size.x;
                    let c = vec2(self.rect_size.x, self.rect_size.y);
                    let pad = 0.35;
                    sdf.rect(0., 0., self.rect_size.x, self.rect_size.y)

                    sdf.fill_keep(
                        mix(
                            mix((COLOR_PANEL_2), (COLOR_MAGENTA), self.hover * 0.5),
                            (COLOR_HOT),
                            self.active
                        )
                    )

                    sdf.stroke_keep(mix((COLOR_UP_4), (COLOR_HILITE), self.active), 2.0);

                    sdf.subtract()

                    let padx = c.x * pad;
                    let pady = c.y * pad;

                    sdf.move_to(c.x - sz + padx, c.y - sz + pady);
                    sdf.line_to(c.x - padx, c.y * 0.5);
                    sdf.line_to(0.0 + padx, c.y - pady);
                    sdf.close_path();

                    sdf.fill_keep(mix((COLOR_PALE), (COLOR_HILITE), self.hover + self.active))

                    return sdf.result
                }
            }
        }
    }

    FishCheckbox = <CheckBox> {
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size)

                let left = 1;
                let sz = 6.5;

                let c = vec2(left + sz, self.rect_size.y * 0.5 - 2.0);

                sdf.rect(left, c.y - sz, sz * 2.5, sz * 2.5);
                sdf.fill_keep(mix((COLOR_DOWN_4), (COLOR_MAGENTA), self.active))
                sdf.stroke(mix((COLOR_UP_3), (COLOR_UP_5), self.hover), 2.0) // outline

                let szs = sz * 0.5;
                let dx = 1.0;

                let offset = 1.5;

                sdf.move_to(left + 4.0 + offset, c.y + offset);
                sdf.line_to(c.x + offset, c.y + szs + offset);
                sdf.line_to(c.x + szs + offset, c.y - szs + offset);

                sdf.stroke_keep(mix((COLOR_UP_0), (COLOR_HILITE), self.active), 2.0);

                return sdf.result
            }
        }

        label_walk: {margin: {left: 23.0}}

        draw_text: {
            text_style: <H2_TEXT_BOLD> {},
            fn get_color(self) -> vec4 {
                return (COLOR_UP_5)
            }
        }
    }

    FishInput = <TextInput> {
        width: Fill,
        height: Fit,
        margin: 0

        clip_x: true,
        clip_y: true,
        align: {y: 0.5},
        text: "Search"
        
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                sdf.rect(0.0, 0.0, self.rect_size.x, self.rect_size.y)
                sdf.fill_keep((COLOR_DOWN_4))
                sdf.stroke((COLOR_UP_3), 2.0)
                return sdf.result;
            }
        },
        draw_text: {
            text_style: <H2_TEXT_REGULAR> {},
        }
    }

    PresetFavorite = <CheckBox> {
        height: Fit,
        width: Fit,
        margin: 0.0
        padding: 0.0

        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size)

                let left = 1;
                let sz = self.rect_size.x / 2;
                let c = vec2(sz, sz);

                let csz = 4.0;
                sdf.circle(csz, csz, csz);
                sdf.circle(csz * 3, csz, csz);
                sdf.union();

                let squeeze = sz * 0.025;
                let top_offset = 0.6;
                // let top_offset = 0.6;
                sdf.move_to(c.x - sz + squeeze + 1.0, c.y - (sz * top_offset));
                sdf.line_to(c.x - sz * 0.5, c.y - (sz * 0.8));
                sdf.line_to(c.x - squeeze, c.y - (sz * top_offset));
                sdf.line_to(c.x * 0.5, c.y);
                sdf.close_path();

                sdf.fill_keep(
                    mix(
                        mix((COLOR_DOWN_4), (COLOR_UP_3), self.hover),
                        mix((COLOR_MAGENTA), (COLOR_HOT), self.hover),
                        self.active
                    )
                )


                return sdf.result
            }
        }
        draw_text: {
            text_style: <H2_TEXT_BOLD> {},
            color: (COLOR_UP_6)
        }
    }


    // PANELS
    EnvelopePanel = <SquareView> {
        flow: Down,
        padding: <SPACING_0> {}
        width: Fill,
        height: Fit

        display = <GraphPaper> {}

        <View> {
            width: Fill,
            height: Fit
            flow: Right,
            spacing: (SSPACING_1)
            attack = <InstrumentSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_ENV)}
                    min: 0.0
                    max: 1.0
                    text: "A"
                }
            }

            hold = <InstrumentSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_ENV)}
                    min: 0.0
                    max: 1.0
                    text: "H"
                }
            }

            decay = <InstrumentSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_ENV)}
                    min: 0.0
                    max: 1.0
                    text: "D"
                }
            }

            sustain = <InstrumentSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_ENV)}
                    min: 0.0
                    max: 1.0
                    text: "S"
                }
            }

            release = <InstrumentSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_ENV)}
                    min: 0.0
                    max: 1.0
                    text: "R"
                }
            }

        }

    }

    VolumeEnvelopePanel = <View> {
        vol_env = <EnvelopePanel> {
            flow: Down
            width: Fill,
            height: Fit
        }
    }

    ModEnvelopePanel = <View> {
        width: Fill,
        height: Fit
        flow: Down

            <View> {
            flow: Down
            width: Fill,
            height: Fit
                <View> {
                flow: Right,
                align: {x: 0.5, y: 0.0}
                width: Fill,
                height: Fit

                    <SubheaderContainer> {
                    <FishSubTitle> {
                        width: Fill
                        label = {
                            text: "Modulation",
                            draw_text: {color: (COLOR_ENV)},
                        }
                    }
                }

            }
        }

        mod_env = <EnvelopePanel> {
            flow: Down,
            padding: <SPACING_0> {}
            width: Fill,
            height: Fit
        }

        modamount = <InstrumentBipolarSlider> {
            width: Fill
            slider = {
                draw_bg: {line_color: (COLOR_ENV)}
                min: -1.0
                max: 1.0
                text: "Influence on Cutoff"
            }
        }

    }

    // Pixel-art icon button (generated 8-bit assets): a bevelled FishButton
    // with a PixelIcon overlaid.
    PxButton = <View> {
        width: 28, height: 28
        flow: Overlay
        align: {x: 0.5, y: 0.5}
        button = <FishButton> { width: Fill, height: Fill, padding: 0, margin: 0, text: "" }
        icon = <PixelIcon> { width: 20, height: 20 }
    }

    // Game of Life controls: shared ids between the main and the life window
    LifeControls = <View> {
        height: Fit,
        width: Fill
        flow: Right,
        spacing: (SSPACING_1),
        padding: {top: (SSPACING_2), bottom: (SSPACING_0)}
        align: {x: 0.0, y: 0.5}

        <PixelIcon> { source: (PX_CELL), width: 18, height: 18, margin: {right: 2} }
        life = <FishToggle> {
            width: Fit
            checkbox = { text: "Life", width: Fit, padding: {right: (SSPACING_2)} }
        }
        liferate = <InstrumentSlider> {
            width: Fill
            slider = {
                draw_bg: {line_color: (COLOR_HOT)}
                min: 1.0
                max: 16.0
                step: 1.0
                precision: 0
                default: 4.0
                text: "Steps/Gen"
            }
        }
        lifeseed = <PxButton> { icon = { source: (PX_SEED) } }
        lifewindow = <PxButton> { icon = { source: (PX_SPIRAL) } }
    }

    SequencerControls = <View> {
        height: Fit,
        width: Fill,
        margin: <SPACING_1> {}
        flow: Down,
        padding: <SPACING_2> {}

        <View> {
            height: Fit,
            width: Fill
            flow: Right,
            spacing: (SSPACING_1),
            padding: {bottom: (SSPACING_0), top: (SSPACING_0)}
            align: {x: 0.0, y: 0.5}

            rootnote = <InstrumentDropdown> {
                height: Fit,
                width: Fit
                dropdown = {
                    labels: ["A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#"]
                    values: [A, Asharp, B, C, Csharp, D, Dsharp, E, F, Fsharp, G, Gsharp]
                }
            }

            scaletype = <InstrumentDropdown> {
                height: Fit,
                width: Fit
                dropdown = {
                    labels: ["Minor", "Major", "Dorian", "Pentatonic"]
                    values: [Minor, Major, Dorian, Pentatonic]
                }
            }

            <View> {
                width: Fill
            }

            clear_grid = <IconButton> {draw_icon: {svg_file: (ICO_SEQ_SWEEP)} icon_walk: {width: 15.0, height: Fit}}
            grid_up = <IconButton> {draw_icon: {svg_file: (ICO_UP)} icon_walk: {width: 10.0, height: Fit}}
            grid_down = <IconButton> {draw_icon: {svg_file: (ICO_DOWN)} icon_walk: {width: 10.0, height: Fit}}
        }

        <LifeControls> {}
    }

    Arp = <GradientYView> {
        flow: Down,
        padding: <SPACING_0> {}
        spacing: (SSPACING_2)
        height: Fit,
        width: 120,
        margin: <SPACING_0> {}
        draw_bg: {color_1: (COLOR_UP_0), color_2: (COLOR_UP_0)}

        <View> {
            flow: Right,
            align: {x: 0.0, y: 0.0} padding: <SPACING_0> {}
            width: Fill,
            height: Fit,
            margin: <SPACING_0> {}

            <SubheaderContainer> {
                margin: <SPACING_0> {}
                <FishSubTitle> {
                    label = {
                        text: "Arp",
                        draw_text: {color: (COLOR_DEFAULT)},
                    }
                }

                <FillerH> {}

                arp = <FishToggle> {
                    margin: <SPACING_0> {}
                    padding: <SPACING_0> {}
                    checkbox = {
                        text: " "
                        padding: {top: (SSPACING_0), right: (SSPACING_1), bottom: (SSPACING_0), left: (SSPACING_0)}
                        margin: <SPACING_0> {}
                    }
                    width: Fit,
                    height: Fit,
                    margin: <SPACING_0> {}
                }
            }


        }

        arpoctaves = <InstrumentBipolarSlider> {
            width: Fill,
            margin: <SPACING_0> {}
            padding: <SPACING_0> {}
            slider = {
                draw_bg: {line_color: (COLOR_DEFAULT)}
                min: -4.0
                max: 4.0
                step: 1.0
                precision: 0,
                text: "Octaves"
            }
        }
    }

    PianoSettings = <View> {
        flow: Down,
        padding: <SPACING_0> {} spacing: (SSPACING_2)
        height: Fit,
        width: 120,
        margin: <SPACING_0> {}

        <SubheaderContainer> {
            margin: <SPACING_0> {}
            <FishSubTitle> {
                label = {
                    text: "Settings",
                    draw_text: {color: (COLOR_DEFAULT)},
                }
            }
        }

        porta = <InstrumentSlider> {
            width: Fill,
            margin: <SPACING_0> {}
            padding: <SPACING_0> {}
            slider = {
                width: Fill
                draw_bg: {line_color: (COLOR_DEFAULT)}
                min: 0.0
                max: 1.0
                text: "Portamento"
            }
        }
    }

    SequencerPanel = <SquareView> {
        flow: Down
        margin: <SPACING_0> {}

        <FishPanelScrollY> {
            width: Fill,
            height: Fill
            flow: Down,
            spacing: (SSPACING_0),
            padding: {top: (SSPACING_2)}
            draw_bg: {color_1: (COLOR_UP_3), color_2: (COLOR_UP_1)}

            <FishHeader> {
                title = {
                    width: Fill
                    label = {
                        text: "Sequencer",
                    },
                    draw_bg: {color: (COLOR_DEFAULT)}
                }
                menu = {
                    width: Fit
                }
            }

            <FishPanel> {
                height: Fit
                flow: Down
                padding: <SPACING_0> {}
                draw_bg: {
                    color_1: (COLOR_DOWN_3),
                    color_2: (COLOR_DOWN_4)
                    bevel_light: (COLOR_DOWN_5)
                    bevel_dark: (COLOR_UP_3)
                }

                <View> {
                    height: Fit,
                    width: Fill
                    flow: Right,
                    align: {x: 0.0, y: 0.5}
                    spacing: (SSPACING_4),
                    padding: {top: (SSPACING_1), right: (SSPACING_3), bottom: (SSPACING_0), left: (SSPACING_0)}

                    playpause = <CheckBox> {
                        draw_bg: {check_type: None}
                        draw_icon: {svg_file: (ICO_PLAY)}
                        icon_walk: {
                            width: 25.0,
                            height: Fit,
                            margin: 0,
                        }

                        margin: {
                            top: 0,
                            right: -10,
                            bottom: 0,
                            left: 0,
                        }

                        padding: {
                            top: 2,
                            right: 0,
                            bottom: 0,
                            left: 15,
                        }
                    }

                    speed = <InstrumentSlider> {
                        width: Fill
                        slider = {
                            draw_bg: {line_color: (COLOR_DEFAULT)}
                            min: 0.0
                            max: 240.0
                            text: "BPM"
                        }
                    }
                }

                <Divider> {margin: {top: (SSPACING_2), right: (SSPACING_0), bottom: (SSPACING_0)}}

                sequencer = <Sequencer> {width: Fill, height: 272, margin: {top: (SSPACING_2)}}

                <Divider> {margin: {top: (SSPACING_2), right: (SSPACING_0), bottom: (SSPACING_0)}}

                <SequencerControls> {}

            }
        }
    }
    CrushFXPanel = <View> {
        width: Fill,
        height: Fit
        flow: Down

            <View> {
            flow: Right,
            align: {x: 0.0, y: 0.0}
            width: Fill,
            height: Fit

                <SubheaderContainer> {
                margin: {top: (SSPACING_0)}
                <FishSubTitle> {
                    label = {
                        text: "Bitcrush",
                        draw_text: {color: (COLOR_FX)},
                    }
                }

                <FillerV> {}

                crushenable = <FishToggle> {
                    margin: <SPACING_0> {}
                    padding: <SPACING_0> {}
                    checkbox = {
                        text: " "
                        padding: {top: (SSPACING_0), right: (SSPACING_1), bottom: (SSPACING_0), left: (SSPACING_0)}
                        margin: <SPACING_0> {}
                    }
                    width: Fit,
                    height: Fit,
                    margin: {top: (SSPACING_0)}
                }
            }
        }

        <View> {
            width: Fill,
            height: Fit
            crushamount = <InstrumentSlider> {
                width: Fill,
                height: Fit
                slider = {
                    draw_bg: {line_color: (COLOR_FX)}
                    min: 0.0
                    max: 1.0
                    text: "Amount"

                }
            }
        }
    }

    DelayFXPanel = <FishPanelContainer> {
        <SubheaderContainer> {
            <FishSubTitle> {
                label = {
                    text: "Delay",
                    draw_text: {color: (COLOR_FX)},
                }
            }
        }
        <View> {
            flow: Down
            width: Fill,
            height: Fit

                <View> {
                flow: Right,
                spacing: (SSPACING_1)
                width: Fill,
                height: Fit

                delaysend = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Send"
                    }
                }

                delayfeedback = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Feedback"

                    }
                }

            }

            <View> {
                flow: Right,
                spacing: (SSPACING_1)
                width: Fill,
                height: Fit

                delaydifference = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Stereo difference"
                    }
                }

                delaycross = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Crossover"
                    }
                }
            }
            <View> {
                flow: Right,
                spacing: (SSPACING_1)
                width: Fill,
                height: Fit

                delaylength = <InstrumentSlider> {
                    

                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Length"
                    }
                }
                <View>{width: Fill}
            }
        }
    }

    ChorusFXPanel = <FishPanelContainer> {
        <SubheaderContainer> {
            <FishSubTitle> {
                label = {
                    text: "Chorus",
                    draw_text: {color: (COLOR_FX)},
                }
            }
        }
        <View> {
            flow: Down
            width: Fill,
            height: Fit

                <View> {
                flow: Right,
                spacing: (SSPACING_1)
                width: Fill,
                height: Fit

                chorusmix = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Mix"
                    }
                }
                chorusdelay = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Pre"
                    }
                }
            }
            <View> {
                flow: Right,
                spacing: (SSPACING_1)
                width: Fill,
                height: Fit
                chorusmod = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Depth"
                    }
                }
                chorusrate = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Rate"
                    }
                }
            }
            <View> {
                flow: Right,
                spacing: (SSPACING_1)
                width: Fill,
                height: Fit
                chorusphase = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Phasing"
                    }
                }

                chorusfeedback = <InstrumentBipolarSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: -1
                        max: 1
                        text: "Feedback"
                    }
                }
            }

        }
    }

    DelayToyFXPanel = <FishPanelContainer> {
        <SubheaderContainer> {
            <FishSubTitle> {
                label = {
                    text: "Reverb",
                    draw_text: {color: (COLOR_FX)},
                }
            }
        }
        <View> {
            flow: Down
            width: Fill,
            height: Fit

                <View> {
                flow: Right
                width: Fill,
                height: Fit

                reverbmix = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Mix"
                    }
                }
                reverbfeedback = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        text: "Feedback"
                    }
                }
            }
        }
    }

    FishPanelFilter = <FishPanelContainer> {

        <FishPanel> {
            height: Fit

                <FishHeader> {
                draw_bg: {color: (COLOR_FILTER)}
                align: { x: 0.0, y: 0.5 }
                title = {
                    width: Fit
                    label = {
                        text: "Filter",
                    },
                }

                menu = <View> {
                    height: Fit,
                    filter_type = <FishDropDown> {
                        width: Fill,

                        labels: ["LowPass", "HighPass", "BandPass", "BandReject"]
                        values: [LowPass, HighPass, BandPass, BandReject]

                        draw_text: {
                            instance down: 0.0,
                            text_style: <H2_TEXT_REGULAR> {},
                            fn get_color(self) -> vec4 {
                                return mix(
                                    mix(
                                        mix(
                                            (COLOR_PALE),
                                            (COLOR_PALE),
                                            self.focus
                                        ),
                                        (COLOR_HILITE),
                                        self.hover
                                    ),
                                    (COLOR_HOT),
                                    self.down
                                )
                            }
                        }

                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                self.get_bg(sdf);
                                // triangle
                                let c = vec2(self.rect_size.x - 10.0, self.rect_size.y * 0.5)
                                let sz = 2.5;

                                sdf.move_to(c.x - sz, c.y - sz);
                                sdf.line_to(c.x + sz, c.y - sz);
                                sdf.line_to(c.x, c.y + sz * 0.75);
                                sdf.close_path();

                                sdf.fill(mix((COLOR_PALE), (COLOR_HILITE), self.hover));

                                return sdf.result
                            }

                            fn get_bg(self, inout sdf: Sdf2d) {
                                sdf.rect(
                                    0,
                                    0,
                                    self.rect_size.x,
                                    self.rect_size.y
                                )
                                sdf.fill((COLOR_UP_0))
                            }
                        }

                        popup_menu: {
                            menu_item: {
                                indent_width: 10.0
                                width: Fill,
                                height: Fit

                                padding: {left: (SSPACING_4), top: (SSPACING_2), bottom: (SSPACING_2), right: (SSPACING_2)}
                            }
                        }

                    }
                }
            }

            <View> {
                flow: Right,
                spacing: (SSPACING_1)
                width: Fill,
                height: Fit
                cutoff = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FILTER)}
                        min: 0.0
                        max: 1.0
                        text: "Cutoff"
                    }
                }

                resonance = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FILTER)}
                        min: 0.0
                        max: 1.0
                        text: "Resonance"
                    }
                }
            }
            <View> {
                flow: Right,
                spacing: (SSPACING_1)
                width: Fill,
                height: Fit

                lfoamount = <InstrumentBipolarSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FILTER)}
                        min: -1.0
                        max: 1.0
                        text: "Cutoff LFO Amount"
                    }
                }
                rate = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_FILTER)}
                        max: 1.0
                        text: "Cutoff LFO Rate"
                    }
                }
            }

            sync = <FishToggle> { checkbox = {width: 200, text: "LFO Key sync"}}
        }
    }

    OscPanel = <View> {
        width: Fill,
        height: Fit
        flow: Down
        <View> {
            flow: Right
            width: Fill,
            height: Fit

            <SubheaderContainer> {
                <FishSubTitle> {label = {text: "Osc", draw_text: {color: (COLOR_OSC)}, width: Fit}}
                type = <InstrumentDropdown> {
                    flow: Down
                    dropdown = {
                        width: Fill,
                        height: Fit
                        values: [DPWSawPulse, BlampTri, Pure, SuperSaw, HyperSaw, HarmonicSeries]
                        labels: ["Saw", "Triangle", "Sine", "Super Saw", "Hyper Saw", "Harmonic"]
                    }
                }
            }
        }

        twocol = <View> {
            flow: Down
            width: Fill,
            height: Fit
            transpose = <InstrumentBipolarSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_OSC)}
                    min: -24.0
                    max: 24.0
                    step: 1.0
                    precision: 0,
                    text: "Transpose"
                }
            }

            detune = <InstrumentBipolarSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_OSC)}
                    min: -1.0
                    max: 1.0
                    text: "Detune"
                }
            }
        }

        <View> {
            flow: Down
            width: Fill,
            height: Fit
            supersaw = <View> {
                flow: Down
                width: Fill,
                height: Fit
                spread = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_OSC)}
                        min: 0.0
                        max: 1.0
                        text: "Spread"
                    }
                }
                diffuse = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_OSC)}
                        min: 0.0
                        max: 1.0
                        text: "Diffuse"
                    }
                }
            }


            hypersaw = <View> {
                flow: Down
                width: Fill,
                height: Fit
                spread = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_OSC)}
                        min: 0.0
                        max: 1.0
                        text: "Spread"
                    }
                }
                diffuse = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_OSC)}
                        min: 0.0
                        max: 1.0
                        text: "Diffuse"
                    }
                }
            }

            harmonic = <View> {
                flow: Down
                width: Fill,
                height: Fit
                harmonicshift = <InstrumentSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_OSC)}
                        min: 0
                        max: 1.0
                        text: "Shift"
                    }
                }
                harmonicenv = <InstrumentBipolarSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_OSC)}
                        min: -1.0
                        max: 1.0
                        text: "Env mod"
                    }
                }
                harmoniclfo = <InstrumentBipolarSlider> {
                    slider = {
                        draw_bg: {line_color: (COLOR_OSC)}
                        min: -1.0
                        max: 1.0
                        text: "LFO mod"
                    }
                }
            }
        }
    }

    MixerPanel = <View> {
        width: Fill,
        height: Fit
        flow: Down
            <View> {
            flow: Right,
            spacing: (SSPACING_1)
            width: Fill,
            height: Fit
            noise = <InstrumentSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_OSC)}
                    min: 0.0
                    max: 1.0
                    text: "Noise"
                }
            }
            sub = <InstrumentSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_OSC)}
                    min: 0.0
                    max: 1.0
                    text: "Sub"
                }
            }
        }
        <View> {
            flow: Right
            width: Fill,
            height: Fit
            balance = <InstrumentBipolarSlider> {
                slider = {
                    draw_bg: {line_color: (COLOR_OSC)}
                    min: 0.0
                    max: 1.0
                    text: "Oscillator Balance"
                }
            }
        }
    }

    FishPanelSoundSources = <FishPanelContainer> {
        width: Fill,
        height: Fill
        padding: <SPACING_0> {}
        spacing: (SSPACING_0)
        flow: Down

            <FishPanelScrollY> {
            height: Fill

                <FishHeader> {
                title = {
                    label = {
                        text: "Sound Sources",
                    },
                    draw_bg: {color: (COLOR_OSC)}
                }
            }

            <SubheaderContainer> {
                margin: {top: (SSPACING_0)}
                <FishSubTitle> {
                    label = {
                        text: "Mixer",
                        draw_text: {color: (COLOR_OSC)},
                    }
                }
            }

            <MixerPanel> {width: Fill, height: Fit}

            <View> {
                width: Fill,
                height: Fit
                flow: Right,
                spacing: (SSPACING_2)

                osc1 = <OscPanel> {}
                osc2 = <OscPanel> {}
            }
            <FillerV> {}
        }
    }

    HeaderMenu = <View> {
        width: Fill,
        height: Fit,
        margin: {top: -150}
        flow: Right,
        spacing: (SSPACING_0),
        align: {x: 0.0, y: 0.0}

        <View> { // TODO: Remove excessive nesting?
            flow: Down,
            align: {x: 0.0, y: 0.0}
            spacing: 0,
            padding: <SPACING_2> {}
            height: 150,
            width: Fill,
            margin: <SPACING_2> {}

            <View> {
                width: Fill
                height: Fit
                flow: Right,
                align: {x: 0.0, y: 0.0}

                <View> {
                    flow: Down,
                    align: {x: 0.0, y: 0.0}
                    <Label> {
                        margin: {bottom: (SSPACING_1), right:10}
                        draw_text: {
                            text_style: <H2_TEXT_BOLD> {},
                            color: (COLOR_UP_5)
                        }
                        text: "Preset"
                    }
                    <View>{
                        width: Fit, height: Fit, flow: Right, spacing: 2
                        preset_1 = <FishButton>{text:"1", width: 24, height: 24, padding: 0, margin: {right: 2}}
                        preset_2 = <FishButton>{text:"2", width: 24, height: 24, padding: 0, margin: {right: 2}}
                        preset_3 = <FishButton>{text:"3", width: 24, height: 24, padding: 0, margin: {right: 2}}
                        preset_4 = <FishButton>{text:"4", width: 24, height: 24, padding: 0, margin: {right: 2}}
                        preset_5 = <FishButton>{text:"5", width: 24, height: 24, padding: 0, margin: {right: 2}}
                        preset_6 = <FishButton>{text:"6", width: 24, height: 24, padding: 0, margin: {right: 2}}
                        preset_7 = <FishButton>{text:"7", width: 24, height: 24, padding: 0, margin: {right: 2}}
                        preset_8 = <FishButton>{text:"8", width: 24, height: 24, padding: 0, margin: {right: 2}}
                    }
                        
                    
                }
                <View> {
                    width: Fill,
                    height: Fit,
                    margin: <SPACING_4> {}
                    spacing: (SSPACING_1)
                }

                logo = <PixelLogo> {}

            }

            <FillerV> {}

            <View> {
                width: Fill,
                height: 35
                spacing: (SSPACING_1)


                prev = <IconButton> {draw_icon: {svg_file: (ICO_PREV)} icon_walk: {width: Fit, height: 11.0}, margin: {top: 3.25, right: -10.0, bottom: 0.0, left: 0.0}}
                presets = <IconButton> {draw_icon: {svg_file: (ICO_PRESET)} icon_walk: {width: Fit, height: 17.5}, margin: 0.0}
                next = <IconButton> {draw_icon: {svg_file: (ICO_NEXT)}, icon_walk: {width: Fit, height: 11.0}, margin: {top: 3.25, right: 0.0, bottom: 0.0, left: -10.0}}

                panic = <IconButton> {draw_icon: {svg_file: (ICO_PANIC)} icon_walk: {width: Fit, height: 17.0}, margin: {left: 5.0, right: -10.0}}
                platformtoggle = <IconButton> {draw_icon: {svg_file: (ICO_PLAT_MOBILE)} icon_walk: {width: Fit, height: 18.5}}

                gitlink = <Label> {
                    draw_text: {text_style: <H2_TEXT_REGULAR> {}, color: (COLOR_UP_5)}
                    text: "Made with Makepad\ngithub.com/makepad/makepad"
                    margin: {top: 7.5, left: 5.0}
                }

                <FillerH> {}

                undo = <IconButton> {draw_icon: {svg_file: (ICO_UNDO)} icon_walk: {width: Fit, height: 15.0}, margin: {top: 3.25, right: -5.0, bottom: 0.0, left: 0.0}}
                redo = <IconButton> {draw_icon: {svg_file: (ICO_REDO)} icon_walk: {width: Fit, height: 15.0}, margin: {top: 3.25, right: 0.0, bottom: 0.0, left: -5.0}}
            }

        }

    }

    // Voice control: mic -> notes + parameter chaos + Life seeding.
    VoiceSettings = <View> {
        flow: Right,
        padding: <SPACING_0> {} spacing: (SSPACING_2)
        height: Fit,
        width: 300,
        margin: {left: (SSPACING_2)}

        <View> {
            flow: Down,
            width: Fill, height: Fit
            spacing: (SSPACING_2)

            <SubheaderContainer> {
                margin: <SPACING_0> {}
                <PixelIcon> { source: (PX_MIC), width: 16, height: 16, margin: {left: 2, right: 4} }
                <FishSubTitle> {
                    label = {
                        text: "Voice",
                        draw_text: {color: (COLOR_DEFAULT)},
                    }
                }
                <FillerH> {}
                voice = <FishToggle> {
                    margin: <SPACING_0> {}
                    padding: <SPACING_0> {}
                    checkbox = {
                        text: " "
                        padding: {top: (SSPACING_0), right: (SSPACING_1), bottom: (SSPACING_0), left: (SSPACING_0)}
                        margin: <SPACING_0> {}
                    }
                    width: Fit,
                    height: Fit,
                    margin: <SPACING_0> {}
                }
            }

            voicesens = <InstrumentSlider> {
                width: Fill,
                margin: <SPACING_0> {}
                padding: <SPACING_0> {}
                slider = {
                    draw_bg: {line_color: (COLOR_HOT)}
                    min: 0.0
                    max: 1.0
                    default: 0.5
                    text: "Sensitivity"
                }
            }
            voicechaos = <InstrumentSlider> {
                width: Fill,
                margin: <SPACING_0> {}
                padding: <SPACING_0> {}
                slider = {
                    draw_bg: {line_color: (COLOR_FX)}
                    min: 0.0
                    max: 1.0
                    default: 0.35
                    text: "Chaos"
                }
            }
        }

        // the real photos: cat while listening, Roy while a note sounds
        <View> {
            width: 54, height: 96
            flow: Overlay
            margin: {top: 2}
            portrait_cat = <View> {
                width: Fill, height: Fill
                <PixelImage> {
                    width: Fill, height: Fill
                    fit: Stretch
                    source: (PHOTO_CAT)
                    draw_bg: { grid: 40.0, tint: 0.5 }
                }
            }
            portrait_roy = <View> {
                visible: false
                width: Fill, height: Fill
                <PixelImage> {
                    width: Fill, height: Fill
                    fit: Stretch
                    source: (PHOTO_ROY)
                    draw_bg: { grid: 40.0, tint: 0.5 }
                }
            }
        }
    }


    // TABS
    FishPanelEnvelopes = <FishPanelContainer> {
        width: Fill,
        height: Fill
        padding: <SPACING_0> {}
        align: {x: 0.0, y: 0.0},
        spacing: (SSPACING_0),
        flow: Down

            <FishPanelScrollY> {

            <FishHeader> {
                title = {
                    label = {
                        text: "Envelopes",
                    },
                    draw_bg: {color: (COLOR_ENV)}
                }
            }

            <SubheaderContainer> {
                margin: {top: (SSPACING_0)}
                <FishSubTitle> {
                    label = {
                        text: "Volume",
                        draw_text: {color: (COLOR_ENV)},
                    }
                }
            }

            <VolumeEnvelopePanel> {
                flow: Down
                width: Fill,
                height: Fit
            }

            <ModEnvelopePanel> {
                flow: Down,
                clip_y: true
                width: Fill,
                height: Fit
            }
        }
    }

    Play = <FishPanel> {
        flow: Right,
        padding: {top: (SSPACING_3)}
        spacing: (SSPACING_0)
        height: Fit,
        width: Fill,
        margin: {top: (SSPACING_0), right: (SSPACING_3), bottom: (SSPACING_3), left: (SSPACING_3)}
        draw_bg: {color_1: (COLOR_UP_3), color_2: (COLOR_UP_1)}

        <Arp> {}
        <ScrollXView> {
            piano = <Piano> {height: Fit, width: Fill, margin: {top: (SSPACING_0), right: (SSPACING_2); bottom: (SSPACING_3), left: (SSPACING_2)}}
        }
        <PianoSettings> {}
        <VoiceSettings> {}
    }


    FishPanelEffects = <FishPanelContainer> {
        width: Fill,
        height: Fill
        padding: <SPACING_0> {}
        align: {x: 0.0, y: 0.0},
        spacing: (SSPACING_0),
        flow: Down

            <FishPanelScrollY> {

            <FishHeader> {
                title = {
                    label = {
                        text: "Effects",
                    },
                    draw_bg: {color: (COLOR_FX)}
                }
            }

            width: Fill,
            height: Fill
                <CrushFXPanel> {}
            <ChorusFXPanel> {}
            <DelayToyFXPanel> {}
            <DelayFXPanel> {}
        }
    }
 
    PresetHeader = <View> {
        width: Fill,
        height: Fit,
        margin: {top: 0, right: (SSPACING_4), bottom: 0, left: (SSPACING_4)}
        flow: Down,
        spacing: (SSPACING_2),
        padding: 0

            <SubheaderContainer> {
            <FishSubTitle> {
                width: Fill
                label = {
                    text: "Browse",
                    draw_text: {color: (COLOR_UP_6)}
                }
            }

            <FillerH> {}
            <CheckboxTextual> {text: "Synth", width: Fit}
            <CheckboxTextual> {text: "Seq", width: Fit}
            <CheckboxTextual> {text: "Fav", width: Fit}
        }

        <FishInput> {}

    }
    
    pub AppDesktop = <View>{
        flow: Right
        width: Fill,
        height: Fill
        // padding: <SPACING_0> { align: {x: 0.0, y: 0.0}, spacing: (SSPACING_0), flow: Down}
        show_bg: true,
        draw_bg:{
            fn pixel(self)->vec4{
                return (COLOR_BG)
            }
        }
        <View> {
            width: Fill,
            height: Fill
            padding: <SPACING_0> {}
            align: {x: 0.0, y: 0.0},
            spacing: (SSPACING_0),
            flow: Down
            // APPLICATION HEADER
            <GradientYView> {
                width: Fill,
                height: (HEIGHT_AUDIOVIZ)
                draw_bg: {color_1: (COLOR_VIZ_1), color_2: (COLOR_VIZ_2)}
                display_audio = <DisplayAudio> {
                    height: Fill,
                    width: Fill
                    draw_wave: {
                        fn vu_fill(self) -> vec4 {return #0000}
                        // 8-bit oscilloscope: waveform quantized to PX-sized blocks, pink ramp
                        fn pixel(self) -> vec4 {
                            let px = (PX);
                            let qpos = floor(self.pos * self.rect_size / px) * px;
                            let sdf = Sdf2d::viewport(qpos);
                            let step = 0.0;
                            let half = self.rect_size.y * 0.5;
                            for i in 0..4 {
                                let wave = sample2d(self.wave_texture, vec2(qpos.x / self.rect_size.x, step));
                                let right = (wave.y + wave.z / 256.0 - 0.5) * 3.0;
                                let left = (wave.w + wave.x / 256.0 - 0.5) * 3.0;
                                let audio = clamp((left + right) * 0.5 - 0.01, -0.45, 0.45);
                                let audio = floor(audio * half / px) * px / half;
                                let thick = abs(audio) * half * 2.0;
                                sdf.hline(half + audio * half * 2.0, max(thick, px) * step(0.001, thick));
                                let t = step * 4.0;
                                let color = mix((COLOR_HOT), (COLOR_FX), t);
                                sdf.fill(vec4(color.xyz, 0.35 - t * 0.2))
                                step += 1.0 / 16.0;
                            }
                            return sdf.result
                        }
                    }
                }
            }

            <HeaderMenu> {
            }

            // CONTROLS
            <View> {
                width: Fill,
                height: Fill
                flow: Right,
                spacing: (SSPACING_1),
                padding: <SPACING_3> {}
                oscillators = <FishPanelSoundSources> {}
                <View> {
                    flow: Down,
                    spacing: (SSPACING_1)
                    height: Fill,
                    width: Fill
                    envelopes = <FishPanelEnvelopes> {}
                    <FishPanelFilter> {}
                }
                effects = <FishPanelEffects> {}
                <SequencerPanel> {height: Fill, width: Fill}
            }

            <Play> {}
        }
    }

    // Secondary window: the Life grid, big, with a fractal mode.
    pub LifeWindow = <View> {
        flow: Overlay
        width: Fill,
        height: Fill

        // dimmed pixel-hearts backdrop
        <Image> {
            width: Fill, height: Fill
            source: (PX_BG)
            fit: Stretch
            draw_bg: {
                fn get_color(self) -> vec4 {
                    let uv = fract(self.pos * vec2(2.0, 2.2));
                    let c = sample2d(self.image, uv);
                    return vec4(mix((COLOR_BG).xyz, c.xyz, 0.16), 1.0);
                }
            }
        }

        <View> {
        flow: Down
        width: Fill,
        height: Fill
        padding: <SPACING_3> {}
        spacing: (SSPACING_2)

        <View> {
            width: Fill, height: Fit
            flow: Right
            align: {x: 0.0, y: 0.5}
            spacing: (SSPACING_2)
            <PixelLogoSmall> {}
            <FillerH> {}
            life_mode = <View> {
                width: Fit, height: Fit
                flow: Right
                spacing: 2
                tab_grid = <FishButton> { text: "GRID", width: 72, height: 26, padding: 0 }
                tab_fractal = <FishButton> { text: "FRACTAL", width: 96, height: 26, padding: 0 }
            }
        }

        life_pages = <View> {
            width: Fill, height: Fill
            flow: Overlay
            grid_page = <FishPanel> {
                width: Fill, height: Fill
                align: {x: 0.5, y: 0.5}
                sequencer = <Sequencer> {
                    width: 608, height: 608,
                    margin: 0
                    button_size: vec2(38.0, 38.0)
                }
            }
            fractal_page = <View> {
                visible: false
                width: Fill, height: Fill
                fractal = <LifeFractal> { width: Fill, height: Fill }
            }
        }

        <FishPanel> {
            width: Fill, height: Fit
            <LifeControls> {}
            <View> {
                width: Fill, height: Fit
                flow: Right
                align: {x: 0.0, y: 0.5}
                spacing: (SSPACING_1)
                zoomspeed = <InstrumentSlider> {
                    width: Fill
                    slider = {
                        draw_bg: {line_color: (COLOR_FX)}
                        min: 0.0
                        max: 1.0
                        default: 0.3
                        text: "Zoom speed"
                    }
                }
            }
        }
        }
    }
}
