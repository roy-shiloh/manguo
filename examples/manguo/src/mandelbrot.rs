//! Deep-zoom Mandelbrot for the Life window's FRACTAL mode.
//!
//! Port of makepad's `fractal_zoom` example to stable Rust (scalar f64 tiles
//! on a thread pool, no SIMD), recolored to the pink 8-bit palette, with an
//! auto-dive camera that keeps zooming into a list of interesting points,
//! sped up by the voice level. Every Life generation shifts the palette.

use crate::makepad_widgets::*;

live_design! {
    use link::shaders::*;

    DrawTile = {{DrawTile}} {
        texture tex: texture2d
        fn pixel(self) -> vec4 {
            let fractal = sample2d(self.tex, self.pos)
            // unpack iteration count and magnitude squared from the u32 buffer
            let iter = fractal.y * 65535 + fractal.z * 255;
            let magsq = (fractal.w * 256 + fractal.x - 127);
            let bg = (#x1A0A14).xyz;
            if (iter > self.max_iter) {
                return vec4(bg * (1.0 - self.fade), 1.0);
            }
            let index = abs((iter / self.max_iter * 18.0) - 0.01 * log(magsq));
            // banded pink ramp, 12 steps
            let t = fract(index + self.color_cycle);
            let t = floor(t * 12.0) / 12.0;
            let c1 = mix(bg, (#x6A2A55).xyz, smoothstep(0.0, 0.3, t));
            let c2 = mix(c1, (#xE0218A).xyz, smoothstep(0.3, 0.55, t));
            let c3 = mix(c2, (#xFFE1F0).xyz, smoothstep(0.55, 0.75, t));
            let c4 = mix(c3, (#xD98CFF).xyz, smoothstep(0.75, 0.9, t));
            let col = mix(c4, bg, smoothstep(0.9, 1.0, t));
            return vec4(col * (1.0 - self.fade), 1.0);
        }
    }

    pub Mandelbrot = {{Mandelbrot}} {
        max_iter: 1024,
    }
}

pub const TILE_SIZE_X: usize = 256;
pub const TILE_SIZE_Y: usize = 256;
pub const TILE_CACHE_SIZE: usize = 300;
const MIN_ZOOM: f64 = 3e-13; // f64 precision floor for 256px tiles
const MAX_ZOOM: f64 = 2.0;

#[derive(Live, LiveHook, LiveRegister)]
#[repr(C)]
pub struct DrawTile {
    #[deref] draw_super: DrawQuad,
    #[live] max_iter: f32,
    #[live] color_cycle: f32,
    #[live] fade: f32,
}

fn mandelbrot_pixel_f64(max_iter: usize, c_x: f64, c_y: f64) -> (usize, f64) {
    let mut x = c_x;
    let mut y = c_y;
    let mut magsq = 0.0;
    for n in 0..max_iter {
        let xy = x * y;
        let xx = x * x;
        let yy = y * y;
        magsq = xx + yy;
        if magsq > 4.0 {
            return (n, magsq);
        }
        x = (xx - yy) + c_x;
        y = (xy + xy) + c_y;
    }
    (max_iter, magsq)
}

fn mandelbrot_f64(tile: &mut Tile, max_iter: usize) {
    let tile_size = dvec2(TILE_SIZE_X as f64, TILE_SIZE_Y as f64);
    for y in 0..TILE_SIZE_Y {
        for x in 0..TILE_SIZE_X {
            let fp = tile.fractal.pos + tile.fractal.size * (dvec2(x as f64, y as f64) / tile_size);
            let (iter, dist) = mandelbrot_pixel_f64(max_iter, fp.x, fp.y);
            let dist = ((dist + 127.0) * 256.0).max(0.0).min(65535.0) as u32;
            tile.buffer[y * TILE_SIZE_X + x] = iter as u32 | (dist << 16);
        }
    }
}

pub struct Tile {
    pub buffer: Vec<u32>,
    pub texture_index: usize,
    pub fractal: Rect,
}

impl Tile {
    fn new(texture_index: usize) -> Self {
        Self { buffer: vec![0; TILE_SIZE_X * TILE_SIZE_Y], texture_index, fractal: Rect::default() }
    }
}

#[derive(Clone, Default)]
pub struct BailTest {
    space: Rect,
    is_zoom_in: bool,
}

pub struct TileCache {
    textures: Vec<Texture>,
    current: Vec<Tile>,
    next: Vec<Tile>,
    empty: Vec<Tile>,
    current_zoom: f64,
    next_zoom: f64,
    tiles_in_flight: usize,
    thread_pool: MessageThreadPool<BailTest>,
}

impl TileCache {
    fn new(cx: &mut Cx) -> Self {
        let mut empty = Vec::new();
        let mut textures = Vec::new();
        for i in 0..TILE_CACHE_SIZE {
            empty.push(Tile::new(i));
            textures.push(Texture::new_with_format(cx, TextureFormat::VecBGRAu8_32 {
                data: Some(vec![]),
                width: TILE_SIZE_X,
                height: TILE_SIZE_Y,
                updated: TextureUpdated::Full,
            }));
        }
        let use_cores = cx.cpu_cores().max(3) - 2;
        Self {
            textures,
            current: Vec::new(),
            next: Vec::new(),
            empty,
            current_zoom: 0.0,
            next_zoom: 0.0,
            tiles_in_flight: 0,
            thread_pool: MessageThreadPool::new(cx, use_cores),
        }
    }

    fn tile_completed(&mut self, cx: &mut Cx, mut tile: Tile) {
        self.tiles_in_flight -= 1;
        self.textures[tile.texture_index].swap_vec_u32(cx, &mut tile.buffer);
        self.next.push(tile)
    }

    fn tile_bailed(&mut self, tile: Tile) {
        self.tiles_in_flight -= 1;
        self.empty.push(tile);
    }

    fn set_bail_test(&self, bail_test: BailTest) {
        self.thread_pool.send_msg(bail_test);
    }

    fn tile_needs_to_bail(tile: &Tile, bail_window: Option<BailTest>) -> bool {
        if let Some(bail) = bail_window {
            if bail.is_zoom_in && !tile.fractal.intersects(bail.space) {
                return true;
            }
        }
        false
    }

    fn generate_completed(&self) -> bool {
        self.tiles_in_flight == 0
    }

    fn discard_next_layer(&mut self, cx: &mut Cx) {
        while let Some(mut tile) = self.next.pop() {
            self.textures[tile.texture_index].swap_vec_u32(cx, &mut tile.buffer);
            self.empty.push(tile);
        }
    }

    fn discard_current_layer(&mut self, cx: &mut Cx) {
        while let Some(mut tile) = self.current.pop() {
            self.textures[tile.texture_index].swap_vec_u32(cx, &mut tile.buffer);
            self.empty.push(tile);
        }
        self.current_zoom = self.next_zoom;
        std::mem::swap(&mut self.current, &mut self.next);
    }

    fn discard_all(&mut self, cx: &mut Cx) {
        self.discard_next_layer(cx);
        self.discard_current_layer(cx);
        self.discard_current_layer(cx);
    }

    pub fn generate_tasks_and_flip_layers(&mut self, cx: &mut Cx, zoom: f64, center: DVec2, window: Rect, is_zoom_in: bool) -> Vec<Tile> {
        let size = dvec2(zoom, zoom);
        if is_zoom_in && !self.next.is_empty() && self.next[0].fractal.size.x < 0.8 * zoom {
            self.discard_next_layer(cx);
        } else {
            self.discard_current_layer(cx);
        }
        self.next_zoom = zoom;
        let mut render_tasks = Vec::new();
        let window = window.add_margin(size);
        Self::spiral_walk(|_step, i, j| {
            let fractal = Rect { pos: center + size * dvec2(i as f64, j as f64) - 0.5 * size, size };
            if window.intersects(fractal) {
                if let Some(mut tile) = self.empty.pop() {
                    tile.fractal = fractal;
                    render_tasks.push(tile);
                }
                true
            } else {
                false
            }
        });
        self.tiles_in_flight = render_tasks.len();
        render_tasks
    }

    fn spiral_walk<F: FnMut(usize, isize, isize) -> bool>(mut f: F) {
        let mut di = 1;
        let mut dj = 0;
        let mut seg_len = 1;
        let mut i = 0;
        let mut j = 0;
        let mut seg_pass = 0;
        let mut any_intersect = false;
        let mut intersect_step = 0;
        for step in 0..100000 {
            if f(step, i, j) {
                any_intersect = true;
            }
            i += di;
            j += dj;
            seg_pass += 1;
            if seg_len == seg_pass {
                seg_pass = 0;
                let t = di;
                di = -dj;
                dj = t;
                if dj == 0 {
                    intersect_step += 1;
                    if intersect_step > 2 {
                        if !any_intersect {
                            return;
                        }
                        intersect_step = 0;
                        any_intersect = false;
                    }
                    seg_len += 1;
                }
            }
        }
    }
}

pub enum ToUI {
    TileDone { tile: Tile },
    TileBailed { tile: Tile },
}

#[derive(Default, Clone)]
pub struct FractalSpace {
    view_rect: Rect,
    tile_size: DVec2,
    center: DVec2,
    zoom: f64,
}

impl FractalSpace {
    fn new(center: DVec2, zoom: f64) -> Self {
        Self { center, zoom, ..Self::default() }
    }

    fn other(&self, other_zoom: f64, other_center: DVec2) -> Self {
        Self { center: other_center, zoom: other_zoom, ..self.clone() }
    }

    fn fractal_to_screen(&self, pos: DVec2) -> DVec2 {
        let view_center = self.view_rect.pos + self.view_rect.size * 0.5;
        (((pos - self.center) / self.zoom) * self.tile_size) + view_center
    }

    fn screen_to_fractal(&self, pos: DVec2) -> DVec2 {
        let view_center = self.view_rect.pos + self.view_rect.size * 0.5;
        (((pos - view_center) / self.tile_size) * self.zoom) + self.center
    }

    fn fractal_to_screen_rect(&self, rect: Rect) -> Rect {
        let pos1 = self.fractal_to_screen(rect.pos);
        let pos2 = self.fractal_to_screen(rect.pos + rect.size);
        Rect { pos: pos1, size: pos2 - pos1 }
    }

    fn screen_to_fractal_rect(&self, rect: Rect) -> Rect {
        let pos1 = self.screen_to_fractal(rect.pos);
        let pos2 = self.screen_to_fractal(rect.pos + rect.size);
        Rect { pos: pos1, size: pos2 - pos1 }
    }

    /// Zooms around a screen point; returns false when the f64 floor is hit.
    fn zoom_around(&mut self, factor: f64, around: DVec2) -> bool {
        let fpos1 = self.screen_to_fractal(around);
        self.zoom *= factor;
        let mut ok = true;
        if self.zoom < MIN_ZOOM {
            self.zoom = MIN_ZOOM;
            ok = false;
        }
        if self.zoom > MAX_ZOOM {
            self.zoom = MAX_ZOOM;
        }
        let fpos2 = self.screen_to_fractal(around);
        self.center += fpos1 - fpos2;
        ok
    }

    fn view_rect_to_fractal(&self) -> Rect {
        self.screen_to_fractal_rect(self.view_rect)
    }
}

/// Places worth diving into. (x, y) in the complex plane.
const TARGETS: &[(f64, f64)] = &[
    (-0.743643887037158704752191506114774, 0.131825904205311970493132056385139), // seahorse valley spiral
    (-0.77568377, 0.13646737),                                                   // seahorse valley
    (0.28693186889504513, 0.014286693904085048),                                 // elephant valley
    (-1.7499999999999999, 0.0),                                                  // minibrot on the antenna
    (-0.1011, 0.9563),                                                           // top bulb filaments
    (-1.25066, 0.02012),                                                         // scepter valley
    (-0.5503, 0.6259),                                                           // double spiral
];

#[derive(Live, Widget)]
pub struct Mandelbrot {
    #[live] draw_tile: DrawTile,
    #[live] max_iter: usize,
    #[redraw] #[rust] view_area: Area,
    #[walk] walk: Walk,
    #[rust] next_frame: NextFrame,
    #[rust] finger_abs: DVec2,
    #[rust] finger_zooming: bool,
    #[rust(true)] is_zoom_in: bool,
    #[rust(FractalSpace::new(dvec2(TARGETS[0].0, TARGETS[0].1), MAX_ZOOM))] space: FractalSpace,
    #[rust] had_first_draw: bool,
    #[rust(TileCache::new(cx))] tile_cache: TileCache,
    #[rust] to_ui: ToUIReceiver<ToUI>,

    // auto-dive
    #[rust] running: bool,
    #[rust(0.3_f32)] speed: f32,
    #[rust] voice_level: f32,
    #[rust] target: usize,
    #[rust] last_time: f64,
    /// 0 = visible, 1 = faded out (during a jump)
    #[rust] fade: f64,
    #[rust] fading_out: bool,
    #[rust] palette_shift: f32,
}

impl LiveHook for Mandelbrot {
    fn after_new_from_doc(&mut self, cx: &mut Cx) {
        self.next_frame = cx.new_next_frame();
    }
}

#[derive(Clone, DefaultNone)]
pub enum MandelbrotAction {
    None,
}

impl Widget for Mandelbrot {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        if let Some(ne) = self.next_frame.is_event(event) {
            if !self.had_first_draw {
                // the space is undefined (tile_size 0) until draw_walk ran once
                self.view_area.redraw(cx);
                self.next_frame = cx.new_next_frame();
                return;
            }
            let dt = if self.last_time == 0.0 { 0.016 } else { (ne.time - self.last_time).clamp(0.0, 0.1) };
            self.last_time = ne.time;

            if self.had_first_draw && self.tile_cache.generate_completed() && self.tile_cache.current.is_empty() {
                self.generate_tiles_around(cx, self.space.zoom, self.space.view_rect.center());
            }

            let mut tiles_received = 0;
            while let Ok(msg) = self.to_ui.receiver.try_recv() {
                match msg {
                    ToUI::TileDone { tile } => {
                        self.tile_cache.tile_completed(cx, tile);
                        if self.tile_cache.generate_completed() && self.tile_cache.next_zoom != self.space.zoom {
                            let zoom = self.space.zoom * if self.is_zooming() { if self.is_zoom_in { 0.8 } else { 2.0 } } else { 1.0 };
                            self.generate_tiles_around(cx, zoom, self.zoom_point());
                        }
                        tiles_received += 1;
                        if tiles_received > 10 {
                            break;
                        }
                    }
                    ToUI::TileBailed { tile } => self.tile_cache.tile_bailed(tile),
                }
            }

            if self.is_zooming() {
                let factor = if self.finger_zooming {
                    if self.is_zoom_in { 0.98 } else { 1.02 }
                } else {
                    // auto-dive: exponential zoom, faster with voice
                    let rate = (0.15 + 1.35 * self.speed as f64) * (1.0 + 2.0 * self.voice_level as f64);
                    (-dt * rate).exp()
                };
                let around = self.zoom_point();
                let ok = self.space.zoom_around(factor, around);
                if !ok && !self.finger_zooming && !self.fading_out {
                    self.fading_out = true;
                }
                if self.tile_cache.generate_completed() {
                    let zoom = self.space.zoom * if self.is_zoom_in { 0.8 } else { 2.0 };
                    self.generate_tiles_around(cx, zoom, around);
                }
            }

            // jump to the next target through a short fade
            if self.fading_out {
                self.fade = (self.fade + dt * 2.0).min(1.0);
                if self.fade >= 1.0 {
                    self.fading_out = false;
                    self.target = (self.target + 1) % TARGETS.len();
                    let t = TARGETS[self.target];
                    self.space.center = dvec2(t.0, t.1);
                    self.space.zoom = MAX_ZOOM;
                    self.tile_cache.discard_all(cx);
                }
            } else if self.fade > 0.0 {
                self.fade = (self.fade - dt * 1.5).max(0.0);
            }

            self.draw_tile.color_cycle = ((ne.time * 0.03) as f32 + self.palette_shift).fract();
            self.draw_tile.fade = self.fade as f32;
            self.view_area.redraw(cx);
            if self.running || self.finger_zooming || self.fade > 0.0 || !self.tile_cache.generate_completed() {
                self.next_frame = cx.new_next_frame();
            }
        }

        match event.hits(cx, self.view_area) {
            Hit::FingerDown(fe) => {
                self.finger_zooming = true;
                self.finger_abs = fe.abs;
                self.is_zoom_in = true;
                cx.set_key_focus(self.view_area);
                self.view_area.redraw(cx);
                self.next_frame = cx.new_next_frame();
            }
            Hit::FingerMove(fe) => {
                self.finger_abs = fe.abs;
            }
            Hit::FingerUp(_) => {
                self.finger_zooming = false;
                self.is_zoom_in = true;
            }
            Hit::FingerScroll(fs) => {
                let factor = if fs.scroll.y > 0.0 { 1.15 } else { 0.87 };
                self.space.zoom_around(factor, fs.abs);
                self.is_zoom_in = factor < 1.0;
                self.next_frame = cx.new_next_frame();
            }
            Hit::KeyDown(k) => {
                if KeyCode::Space == k.key_code {
                    self.fading_out = true;
                }
            }
            _ => (),
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        cx.begin_turtle(walk, Layout::flow_right());
        self.had_first_draw = true;
        self.space.tile_size = dvec2(TILE_SIZE_X as f64, TILE_SIZE_Y as f64) / cx.current_dpi_factor() as f64;
        self.space.view_rect = cx.turtle().rect();
        self.tile_cache.set_bail_test(BailTest { is_zoom_in: self.is_zoom_in, space: self.space.view_rect_to_fractal() });
        self.draw_tile.max_iter = self.iter_budget(self.tile_cache.next_zoom.max(MIN_ZOOM)) as f32;
        for tile in self.tile_cache.current.iter().chain(self.tile_cache.next.iter()) {
            let rect = self.space.fractal_to_screen_rect(tile.fractal);
            self.draw_tile.draw_vars.set_texture(0, &self.tile_cache.textures[tile.texture_index]);
            self.draw_tile.draw_abs(cx, rect);
        }
        cx.end_turtle_with_area(&mut self.view_area);
        DrawStep::done()
    }
}

impl Mandelbrot {
    fn is_zooming(&self) -> bool {
        self.finger_zooming || (self.running && !self.fading_out)
    }

    fn zoom_point(&self) -> DVec2 {
        if self.finger_zooming {
            self.finger_abs
        } else {
            self.space.view_rect.center()
        }
    }

    /// Iteration budget grows with depth: ~256 at the full set, ~2800 at the f64 floor.
    fn iter_budget(&self, zoom: f64) -> usize {
        let depth = (MAX_ZOOM / zoom.max(MIN_ZOOM)).log10().max(0.0);
        (self.max_iter as f64 * 0.25 + 200.0 * depth) as usize
    }

    pub fn render_tile(&mut self, mut tile: Tile, fractal_zoom: f64) {
        let max_iter = self.iter_budget(fractal_zoom);
        let to_ui = self.to_ui.sender();
        self.tile_cache.thread_pool.execute(move |bail_test| {
            if TileCache::tile_needs_to_bail(&tile, bail_test) {
                return to_ui.send(ToUI::TileBailed { tile }).unwrap();
            }
            mandelbrot_f64(&mut tile, max_iter);
            to_ui.send(ToUI::TileDone { tile }).unwrap();
        })
    }

    pub fn generate_tiles_around(&mut self, cx: &mut Cx, zoom: f64, around: DVec2) {
        let other = self.space.other(zoom, self.space.center);
        let center = other.screen_to_fractal(around);
        let window = other.view_rect_to_fractal();
        let is_zoom_in = self.is_zoom_in;
        let tasks = self.tile_cache.generate_tasks_and_flip_layers(cx, zoom, center, window, is_zoom_in);
        for tile in tasks {
            self.render_tile(tile, zoom);
        }
    }

    pub fn set_running(&mut self, cx: &mut Cx, on: bool) {
        if on && !self.running {
            self.last_time = 0.0;
            self.next_frame = cx.new_next_frame();
        }
        self.running = on;
    }
}

impl MandelbrotRef {
    pub fn set_running(&self, cx: &mut Cx, on: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_running(cx, on);
        }
    }
    pub fn set_speed(&self, speed: f32) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.speed = speed;
        }
    }
    pub fn set_voice_level(&self, level: f32) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.voice_level = level;
        }
    }
    /// A Life generation happened: shift the palette.
    pub fn on_generation(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.palette_shift = (inner.palette_shift + 0.07).fract();
        }
    }
}
