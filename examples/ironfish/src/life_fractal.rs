//! Infinite-zoom, self-similar rendering of the Life grid.
//!
//! Every live cell contains a copy of the whole grid; the camera dives into a
//! chosen live cell forever. When the zoom reaches one full level (a cell
//! fills the viewport) the level counter wraps and the target cell becomes
//! the new "whole grid" origin, which is pixel-identical, so the dive is
//! seamless.

use crate::life::{Life, next_u32, H, W};
use crate::makepad_widgets::*;

live_design! {
    use link::shaders::*;

    DrawLifeFractal = {{DrawLifeFractal}} {
        texture grid: texture2d

        fn cell_alive(self, c: vec2) -> float {
            // grid texture is 16x16, alive stored in red
            let uv = (floor(c) + 0.5) / 16.0;
            return step(0.5, sample2d(self.grid, uv).x);
        }

        fn pixel(self) -> vec4 {
            let px = 3.0;
            let p = floor(self.pos * self.rect_size / px) * px;
            let aspect = self.rect_size.x / self.rect_size.y;
            // normalized view coordinates, square, centred
            let v = (p / self.rect_size - 0.5) * vec2(aspect, 1.0);

            // camera: zoom in [0,1) is a fractional level; scale = 16^zoom
            let scale = pow(16.0, self.zoom);
            // grid coordinates covered by the viewport at this level
            let uv = self.center + v * (16.0 / scale);

            let inside = 1.0;
            let cell = uv;
            let col = (#x1A0A14).xyz;

            for i in 0..6 {
                if (inside > 0.5) {
                    let c = floor(cell);
                    // outside the grid: dead
                    let in_grid = step(0.0, c.x) * step(c.x, 15.0) * step(0.0, c.y) * step(c.y, 15.0);
                    let a = self.cell_alive(c) * in_grid;
                    let f = cell - c;
                    // bevelled square inside each cell
                    let edge = step(f.x, 0.06) + step(f.y, 0.06);
                    let dark = step(0.94, f.x) + step(0.94, f.y);
                    let lvl = float(i);
                    let base = mix((#xFF3FA4).xyz, (#xD98CFF).xyz, fract(lvl * 0.37));
                    let dim = mix((#x2B1020).xyz, (#x3A1530).xyz, fract(lvl * 0.5));
                    // inside a live parent the dead cells are that parent's pink, dimmed
                    let parent_tint = mix(dim, col * 0.55, step(0.5, lvl));
                    let body = mix(parent_tint, base, a);
                    if (edge > 0.0) {
                        body = mix(body, (#xFFE1F0).xyz, 0.5 * a + 0.1);
                    }
                    if (dark > 0.0) {
                        body = body * 0.55;
                    }
                    col = body;
                    // only live cells contain the next level
                    inside = a * step(edge + dark, 0.5);
                    cell = f * 16.0;
                }
            }
            // scanlines
            let scan = 0.9 + 0.1 * step(1.5, mod(p.y / px, 3.0));
            let glow = 1.0 + 0.4 * self.voice_level;
            return Pal::premul(vec4(col * scan * glow, 1.0));
        }
    }

    pub LifeFractal = {{LifeFractal}} {
        width: Fill,
        height: Fill
    }
}

#[derive(Live, LiveHook, LiveRegister)]
#[repr(C)]
struct DrawLifeFractal {
    #[deref] draw_super: DrawQuad,
    #[live] zoom: f32,
    #[live] center: Vec2,
    #[live] voice_level: f32,
}

#[derive(Live, Widget)]
pub struct LifeFractal {
    #[walk] walk: Walk,
    #[redraw] #[live] draw_fractal: DrawLifeFractal,
    #[rust(Texture::new(cx))] grid_texture: Texture,
    #[rust] next_frame: NextFrame,
    #[rust] life: Life,
    #[rust] target: (usize, usize),
    #[rust(0.3_f32)] speed: f32,
    #[rust] voice_level: f32,
    #[rust] zoom: f64,
    #[rust([8.0, 8.0])] center: [f64; 2],
    /// centre at zoom 0 for the current level
    #[rust([8.0, 8.0])] start: [f64; 2],
    /// smoothed target position in grid units (cell centre)
    #[rust([8.5, 8.5])] target_pos: [f64; 2],
    #[rust(0xBADC0DE_u32)] seed: u32,
    #[rust] last_time: f64,
    #[rust] running: bool,
}

impl LiveHook for LifeFractal {
    fn after_new_from_doc(&mut self, cx: &mut Cx) {
        self.grid_texture = Texture::new_with_format(cx, TextureFormat::VecBGRAu8_32 {
            data: Some(vec![0; W * H]),
            width: W,
            height: H,
            updated: TextureUpdated::Full,
        });
    }
}

impl Widget for LifeFractal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        if let Some(ne) = self.next_frame.is_event(event) {
            if self.running {
                let dt = if self.last_time == 0.0 { 0.016 } else { (ne.time - self.last_time).clamp(0.0, 0.1) };
                self.last_time = ne.time;
                self.advance(dt);
                self.draw_fractal.redraw(cx);
                self.next_frame = cx.new_next_frame();
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        self.draw_fractal.draw_vars.set_texture(0, &self.grid_texture);
        self.draw_fractal.zoom = self.zoom as f32;
        self.draw_fractal.center = vec2(self.center[0] as f32, self.center[1] as f32);
        self.draw_fractal.voice_level = self.voice_level;
        self.draw_fractal.draw_walk(cx, walk);
        DrawStep::done()
    }
}

impl LifeFractal {
    fn advance(&mut self, dt: f64) {
        let base = 0.05 + 0.6 * self.speed as f64;
        self.zoom += dt * base * (1.0 + 6.0 * self.voice_level as f64);

        // the target may move mid-flight (new generation): glide toward it
        let tx = self.target.0 as f64 + 0.5;
        let ty = self.target.1 as f64 + 0.5;
        let k = (dt * 4.0).min(1.0);
        self.target_pos[0] += (tx - self.target_pos[0]) * k;
        self.target_pos[1] += (ty - self.target_pos[1]) * k;

        if self.zoom >= 1.0 {
            // A full level: the view (1 grid unit around target_pos) equals a
            // zoom-0 view of the interior grid centred at fract(target_pos)*16.
            // Exact, so the wrap is invisible.
            self.zoom -= 1.0;
            self.start = [
                (self.target_pos[0] - self.target_pos[0].floor()) * 16.0,
                (self.target_pos[1] - self.target_pos[1].floor()) * 16.0,
            ];
            self.pick_target();
            self.target_pos = [self.target.0 as f64 + 0.5, self.target.1 as f64 + 0.5];
        }

        // deterministic camera: start (zoom 0) -> target (zoom 1)
        let w = smoothstep(0.0, 1.0, self.zoom);
        self.center[0] = self.start[0] + (self.target_pos[0] - self.start[0]) * w;
        self.center[1] = self.start[1] + (self.target_pos[1] - self.start[1]) * w;
    }

    fn pick_target(&mut self) {
        if let Some(t) = self.life.random_live_cell(&mut self.seed) {
            self.target = t;
        } else {
            let x = (next_u32(&mut self.seed) % W as u32) as usize;
            let y = (next_u32(&mut self.seed) % H as u32) as usize;
            self.target = (x, y);
        }
    }

    pub fn set_grid(&mut self, cx: &mut Cx, life: &Life) {
        self.life = *life;
        let mut buf = self.grid_texture.take_vec_u32(cx);
        if buf.len() != W * H {
            buf = vec![0; W * H];
        }
        for y in 0..H {
            for x in 0..W {
                buf[y * W + x] = if life.get(x, y) { 0xFFFF_FFFF } else { 0xFF00_0000 };
            }
        }
        self.grid_texture.put_back_vec_u32(cx, buf, None);
        // retarget on every generation (the glide keeps it smooth)
        self.pick_target();
        self.draw_fractal.redraw(cx);
    }

    pub fn set_running(&mut self, cx: &mut Cx, on: bool) {
        if on && !self.running {
            self.last_time = 0.0;
            self.next_frame = cx.new_next_frame();
        }
        self.running = on;
    }
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl LifeFractalRef {
    pub fn set_grid(&self, cx: &mut Cx, life: &Life) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_grid(cx, life);
        }
    }
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
}
