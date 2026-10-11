//! Embedded mode on Windows: run a makepad `Cx` inside an `HWND` owned by
//! another application (an audio plugin host, for instance).
//!
//! Nothing here registers window classes, installs timers or pumps a
//! message loop; the embedder owns the window and the loop. It feeds input
//! through the `EmbeddedCx` methods and calls `frame` at its own refresh
//! cadence. Rendering goes through a DXGI swap chain created for the host
//! window. Makepad timers run on `PollTimers`, like the Studio stdin loop.
//!
//! The first `Window` the app creates is bound to the host window; further
//! windows are marked created but never shown.

use {
    crate::{
        area::Area,
        cx::{Cx, OsType},
        cx_api::CxOsOp,
        event::{
            Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
            ScrollEvent, TextInputEvent, WindowGeom, WindowGeomChangeEvent,
        },
        makepad_live_id::*,
        makepad_math::*,
        os::{cx_stdin::PollTimer, windows::d3d11::D3d11Cx},
        pass::CxPassParent,
        thread::SignalToUI,
        window::{CxWindowPool, WindowId},
        windows::Win32::{
            Foundation::HWND,
            Graphics::{
                Direct3D11::{ID3D11RenderTargetView, ID3D11Texture2D},
                Dxgi::{
                    Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
                    IDXGISwapChain1, DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    DXGI_USAGE_RENDER_TARGET_OUTPUT,
                },
            },
        },
    },
    std::{
        cell::{Cell, RefCell},
        rc::Rc,
    },
};

pub struct EmbeddedCx {
    cx: Rc<RefCell<Cx>>,
    d3d11_cx: D3d11Cx,
    swap_chain: IDXGISwapChain1,
    swap_texture: Option<ID3D11Texture2D>,
    render_target_view: Option<ID3D11RenderTargetView>,
    /// the makepad window bound to the host window
    window_id: Option<WindowId>,
    geom: WindowGeom,
    alloc_size: DVec2,
    started: bool,
}

impl EmbeddedCx {
    /// `cx` must be fully set up (modules registered, `live_design` applied,
    /// `init_cx_os` called) but not started. `hwnd` is the host's window;
    /// `inner_size` is in logical points, `dpi_factor` the window's scale.
    pub fn new(cx: Rc<RefCell<Cx>>, hwnd: isize, inner_size: DVec2, dpi_factor: f64) -> Self {
        let d3d11_cx = D3d11Cx::new();
        {
            let mut cx_ref = cx.borrow_mut();
            cx_ref.self_ref = Some(cx.clone());
            cx_ref.os_type = OsType::Windows;
            cx_ref.os.d3d11_device = Some(d3d11_cx.device.clone());
        }
        let pixel_size = inner_size * dpi_factor;
        let sc_desc = DXGI_SWAP_CHAIN_DESC1 {
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            BufferCount: 2,
            Width: (pixel_size.x.max(1.0)) as u32,
            Height: (pixel_size.y.max(1.0)) as u32,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Flags: 0,
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Scaling: DXGI_SCALING_NONE,
            Stereo: false.into(),
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        };
        let (swap_chain, swap_texture, render_target_view) = unsafe {
            let swap_chain = d3d11_cx
                .factory
                .CreateSwapChainForHwnd(&d3d11_cx.device, HWND(hwnd), &sc_desc, None, None)
                .expect("embedded: CreateSwapChainForHwnd failed");
            let swap_texture: ID3D11Texture2D = swap_chain.GetBuffer(0).unwrap();
            let mut render_target_view = None;
            d3d11_cx
                .device
                .CreateRenderTargetView(&swap_texture, None, Some(&mut render_target_view))
                .unwrap();
            (swap_chain, swap_texture, render_target_view)
        };
        let geom = WindowGeom {
            dpi_factor,
            can_fullscreen: false,
            xr_is_presenting: false,
            is_fullscreen: false,
            is_topmost: false,
            position: dvec2(0.0, 0.0),
            inner_size,
            outer_size: inner_size,
        };
        Self {
            cx,
            d3d11_cx,
            swap_chain,
            swap_texture: Some(swap_texture),
            render_target_view,
            window_id: None,
            geom,
            alloc_size: inner_size,
            started: false,
        }
    }

    /// Access to the context between callbacks (parameter pushes etc.).
    pub fn cx(&self) -> Rc<RefCell<Cx>> {
        self.cx.clone()
    }

    fn time_now(cx: &Cx) -> f64 {
        cx.os.stdin_timers.time_now()
    }

    /// Host window size or scale changed.
    pub fn resize(&mut self, inner_size: DVec2, dpi_factor: f64) {
        let old_geom = self.geom.clone();
        self.geom.inner_size = inner_size;
        self.geom.outer_size = inner_size;
        self.geom.dpi_factor = dpi_factor;
        if let Some(window_id) = self.window_id {
            let mut cx = self.cx.borrow_mut();
            cx.windows[window_id].window_geom = self.geom.clone();
            cx.windows[window_id].os_dpi_factor = Some(dpi_factor);
            if let Some(main_pass_id) = cx.windows[window_id].main_pass_id {
                cx.redraw_pass_and_child_passes(main_pass_id);
            }
            cx.call_event_handler(&Event::WindowGeomChange(WindowGeomChangeEvent {
                window_id,
                old_geom,
                new_geom: self.geom.clone(),
            }));
        }
    }

    fn window_id(&self) -> WindowId {
        self.window_id.unwrap_or_else(CxWindowPool::id_zero)
    }

    pub fn mouse_down(&mut self, abs: DVec2, button: MouseButton, modifiers: KeyModifiers) {
        let window_id = self.window_id();
        let mut cx = self.cx.borrow_mut();
        let time = Self::time_now(&cx);
        cx.fingers.process_tap_count(abs, time);
        cx.fingers.mouse_down(button, window_id);
        cx.call_event_handler(&Event::MouseDown(MouseDownEvent {
            abs,
            button,
            window_id,
            modifiers,
            time,
            handled: Cell::new(Area::Empty),
        }));
    }

    pub fn mouse_move(&mut self, abs: DVec2, modifiers: KeyModifiers) {
        let window_id = self.window_id();
        let mut cx = self.cx.borrow_mut();
        let time = Self::time_now(&cx);
        cx.call_event_handler(&Event::MouseMove(MouseMoveEvent {
            abs,
            window_id,
            modifiers,
            time,
            handled: Cell::new(Area::Empty),
        }));
        cx.fingers.cycle_hover_area(live_id!(mouse).into());
        cx.fingers.switch_captures();
    }

    pub fn mouse_up(&mut self, abs: DVec2, button: MouseButton, modifiers: KeyModifiers) {
        let window_id = self.window_id();
        let mut cx = self.cx.borrow_mut();
        let time = Self::time_now(&cx);
        cx.call_event_handler(&Event::MouseUp(MouseUpEvent { abs, button, window_id, modifiers, time }));
        cx.fingers.mouse_up(button);
        cx.fingers.cycle_hover_area(live_id!(mouse).into());
    }

    pub fn scroll(&mut self, abs: DVec2, scroll: DVec2, is_mouse: bool, modifiers: KeyModifiers) {
        let window_id = self.window_id();
        let mut cx = self.cx.borrow_mut();
        let time = Self::time_now(&cx);
        cx.call_event_handler(&Event::Scroll(ScrollEvent {
            abs,
            scroll,
            window_id,
            modifiers,
            handled_x: Cell::new(false),
            handled_y: Cell::new(false),
            is_mouse,
            time,
        }));
    }

    pub fn key_down(&mut self, key_code: KeyCode, is_repeat: bool, modifiers: KeyModifiers) {
        let mut cx = self.cx.borrow_mut();
        let time = Self::time_now(&cx);
        let e = KeyEvent { key_code, is_repeat, modifiers, time };
        cx.keyboard.process_key_down(e.clone());
        cx.call_event_handler(&Event::KeyDown(e));
    }

    pub fn key_up(&mut self, key_code: KeyCode, modifiers: KeyModifiers) {
        let mut cx = self.cx.borrow_mut();
        let time = Self::time_now(&cx);
        let e = KeyEvent { key_code, is_repeat: false, modifiers, time };
        cx.keyboard.process_key_up(e.clone());
        cx.call_event_handler(&Event::KeyUp(e));
    }

    pub fn text_input(&mut self, input: String) {
        let mut cx = self.cx.borrow_mut();
        cx.call_event_handler(&Event::TextInput(TextInputEvent { input, replace_last: false, was_paste: false }));
    }

    pub fn focus(&mut self, got: bool) {
        let mut cx = self.cx.borrow_mut();
        cx.call_event_handler(if got { &Event::AppGotFocus } else { &Event::AppLostFocus });
        if got {
            cx.redraw_all();
        }
    }

    /// One tick of the embedder's refresh loop: platform ops, timers,
    /// signals, draw and present.
    pub fn frame(&mut self) {
        let cx_rc = self.cx.clone();
        let mut cx = cx_rc.borrow_mut();
        if !self.started {
            self.started = true;
            cx.call_event_handler(&Event::Startup);
            cx.redraw_all();
        }
        self.handle_platform_ops(&mut cx);
        if SignalToUI::check_and_clear_ui_signal() {
            cx.handle_media_signals();
            cx.call_event_handler(&Event::Signal);
        }
        if SignalToUI::check_and_clear_action_signal() {
            cx.handle_action_receiver();
        }
        let events = cx.os.stdin_timers.get_dispatch();
        for event in events {
            cx.call_event_handler(&event);
        }
        if cx.handle_live_edit() {
            cx.call_event_handler(&Event::LiveEdit);
            cx.redraw_all();
        }
        cx.handle_networking_events();
        self.handle_platform_ops(&mut cx);
        if let Some(executor) = cx.executor.take() {
            drop(cx);
            executor.run_until_stalled();
            cx = cx_rc.borrow_mut();
            cx.executor = Some(executor);
        }
        let time = Self::time_now(&cx);
        if cx.new_next_frames.len() != 0 {
            cx.call_next_frame_event(time);
        }
        if cx.need_redrawing() {
            cx.call_draw_event();
            cx.hlsl_compile_shaders(&self.d3d11_cx);
        }
        self.handle_repaint(&mut cx, time as f32);
    }

    fn handle_platform_ops(&mut self, cx: &mut Cx) {
        while let Some(op) = cx.platform_ops.pop() {
            match op {
                CxOsOp::CreateWindow(window_id) => {
                    let window = &mut cx.windows[window_id];
                    window.is_created = true;
                    if self.window_id.is_none() {
                        self.window_id = Some(window_id);
                        window.window_geom = self.geom.clone();
                        window.os_dpi_factor = Some(self.geom.dpi_factor);
                    } else {
                        crate::log!("embedded: extra window {:?} is not shown", window_id);
                    }
                }
                CxOsOp::CloseWindow(window_id) => {
                    cx.windows[window_id].is_created = false;
                    if self.window_id == Some(window_id) {
                        self.window_id = None;
                    }
                }
                CxOsOp::StartTimer { timer_id, interval, repeats } => {
                    cx.os.stdin_timers.timers.insert(timer_id, PollTimer::new(interval, repeats));
                }
                CxOsOp::StopTimer(timer_id) => {
                    cx.os.stdin_timers.timers.remove(&timer_id);
                }
                CxOsOp::HttpRequest { request_id, request } => {
                    use crate::os::windows::http::WindowsHttpSocket;
                    WindowsHttpSocket::open(request_id, request, cx.os.network_response.sender.clone());
                }
                CxOsOp::CopyToClipboard(content) => unsafe {
                    crate::os::windows::win32_window::Win32Window::copy_to_clipboard(&content);
                },
                CxOsOp::SetCursor(_cursor) => {
                    // cursors are the embedder's business (baseview has its own)
                }
                CxOsOp::Quit => {}
                _ => {}
            }
        }
    }

    fn resize_buffers(&mut self) {
        if self.alloc_size == self.geom.inner_size {
            return;
        }
        self.alloc_size = self.geom.inner_size;
        self.swap_texture = None;
        self.render_target_view = None;
        let pixel_size = self.geom.inner_size * self.geom.dpi_factor;
        unsafe {
            self.swap_chain
                .ResizeBuffers(
                    2,
                    pixel_size.x.max(1.0) as u32,
                    pixel_size.y.max(1.0) as u32,
                    DXGI_FORMAT_B8G8R8A8_UNORM,
                    0,
                )
                .unwrap();
            let swap_texture: ID3D11Texture2D = self.swap_chain.GetBuffer(0).unwrap();
            let mut render_target_view = None;
            self.d3d11_cx
                .device
                .CreateRenderTargetView(&swap_texture, None, Some(&mut render_target_view))
                .unwrap();
            self.swap_texture = Some(swap_texture);
            self.render_target_view = render_target_view;
        }
    }

    fn handle_repaint(&mut self, cx: &mut Cx, time: f32) {
        let mut passes_todo = Vec::new();
        cx.compute_pass_repaint_order(&mut passes_todo);
        cx.repaint_id += 1;
        for pass_id in &passes_todo {
            cx.passes[*pass_id].set_time(time);
            match cx.passes[*pass_id].parent.clone() {
                CxPassParent::Xr => {}
                CxPassParent::Window(window_id) => {
                    if Some(window_id) != self.window_id {
                        continue;
                    }
                    self.resize_buffers();
                    cx.draw_pass_to_render_target(*pass_id, &self.render_target_view, &self.d3d11_cx);
                    unsafe {
                        let _ = self.swap_chain.Present(0, 0);
                    }
                }
                CxPassParent::Pass(_) | CxPassParent::None => {
                    cx.draw_pass_to_texture(*pass_id, &self.d3d11_cx, None);
                }
            }
        }
    }
}

impl Drop for EmbeddedCx {
    fn drop(&mut self) {
        if self.started {
            let cx = self.cx.clone();
            let mut cx = cx.borrow_mut();
            cx.call_event_handler(&Event::Shutdown);
        }
    }
}
