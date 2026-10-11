//! Embedded mode: run a makepad `Cx` inside a view owned by another
//! application (an audio plugin host, for instance).
//!
//! Unlike `Cx::event_loop`, nothing here touches `NSApplication`, creates
//! `NSWindow`s or installs timers. The embedder owns the view and the event
//! loop; it feeds input through the `EmbeddedCx` methods and calls `frame`
//! at its own refresh cadence. Rendering goes through a `CAMetalLayer`
//! attached to the host view. Makepad timers run on `PollTimers`, like the
//! Studio stdin loop.
//!
//! The first `Window` the app creates is bound to the host view; further
//! windows are marked created but never shown.

use {
    std::{
        cell::{Cell, RefCell},
        ffi::c_void,
        rc::Rc,
        sync::Once,
    },
    makepad_objc_sys::{msg_send, sel, sel_impl},
    crate::{
        makepad_live_id::*,
        makepad_math::*,
        os::{
            apple::{
                apple_classes::init_apple_classes_global,
                apple_sys::*,
                metal::{DrawPassMode, MetalCx},
            },
            cx_stdin::PollTimer,
        },
        area::Area,
        cx::{Cx, OsType},
        cx_api::CxOsOp,
        event::{
            Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseDownEvent, MouseMoveEvent,
            MouseUpEvent, ScrollEvent, TextInputEvent, WindowGeom, WindowGeomChangeEvent,
        },
        pass::CxPassParent,
        thread::SignalToUI,
        window::{CxWindowPool, WindowId},
    },
};

static APPLE_CLASSES_ONCE: Once = Once::new();

pub struct EmbeddedCx {
    cx: Rc<RefCell<Cx>>,
    metal_cx: MetalCx,
    ca_layer: ObjcId,
    host_view: ObjcId,
    /// the makepad window bound to the host view
    window_id: Option<WindowId>,
    geom: WindowGeom,
    cal_size: DVec2,
    started: bool,
}

impl EmbeddedCx {
    /// `cx` must be fully set up (modules registered, `live_design` applied,
    /// `init_cx_os` called) but not started. `ns_view` is the host's NSView;
    /// a `CAMetalLayer` is installed as its layer. `inner_size` is in logical
    /// points, `dpi_factor` is the backing scale of the host window.
    pub fn new(cx: Rc<RefCell<Cx>>, ns_view: *mut c_void, inner_size: DVec2, dpi_factor: f64) -> Self {
        APPLE_CLASSES_ONCE.call_once(|| init_apple_classes_global());
        let metal_cx = MetalCx::new();
        {
            let mut cx_ref = cx.borrow_mut();
            cx_ref.self_ref = Some(cx.clone());
            cx_ref.os_type = OsType::Macos;
            cx_ref.os.metal_device = Some(metal_cx.device);
        }
        let host_view = ns_view as ObjcId;
        let ca_layer: ObjcId = unsafe { msg_send![class!(CAMetalLayer), new] };
        unsafe {
            let () = msg_send![ca_layer, setDevice: metal_cx.device];
            let () = msg_send![ca_layer, setPixelFormat: MTLPixelFormat::BGRA8Unorm];
            let () = msg_send![ca_layer, setPresentsWithTransaction: NO];
            let () = msg_send![ca_layer, setMaximumDrawableCount: 3];
            let () = msg_send![ca_layer, setDisplaySyncEnabled: YES];
            let () = msg_send![ca_layer, setNeedsDisplayOnBoundsChange: YES];
            let () = msg_send![ca_layer, setAutoresizingMask: (1 << 4) | (1 << 1)];
            let () = msg_send![ca_layer, setAllowsNextDrawableTimeout: NO];
            let () = msg_send![ca_layer, setBackgroundColor: CGColorCreateGenericRGB(0.0, 0.0, 0.0, 1.0)];
            let () = msg_send![host_view, setWantsLayer: YES];
            let () = msg_send![host_view, setLayer: ca_layer];
            // layerContentsPlacement = NSViewLayerContentsPlacementTopLeft
            let () = msg_send![host_view, setLayerContentsPlacement: 11];
        }
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
            metal_cx,
            ca_layer,
            host_view,
            window_id: None,
            geom,
            cal_size: dvec2(0.0, 0.0),
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

    /// Host view size or backing scale changed.
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
        // the executor runs the app's async tasks (ui_runner etc.)
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
            cx.mtl_compile_shaders(&self.metal_cx);
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
                    cx.os.http_requests.make_http_request(request_id, request, cx.os.network_response.sender.clone());
                }
                CxOsOp::CancelHttpRequest { request_id } => {
                    cx.os.http_requests.cancel_http_request(request_id);
                }
                CxOsOp::SetCursor(_cursor) => {
                    // cursors are the embedder's business (baseview has its own)
                }
                CxOsOp::Quit => {}
                _ => {}
            }
        }
    }

    fn resize_layer(&mut self) -> bool {
        let cal_size = self.geom.inner_size * self.geom.dpi_factor;
        if self.cal_size != cal_size {
            self.cal_size = cal_size;
            unsafe {
                let () = msg_send![self.ca_layer, setDrawableSize: CGSize { width: cal_size.x, height: cal_size.y }];
                let () = msg_send![self.ca_layer, setContentsScale: self.geom.dpi_factor];
            }
            true
        } else {
            false
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
                    self.resize_layer();
                    let drawable: ObjcId = unsafe { msg_send![self.ca_layer, nextDrawable] };
                    if drawable == nil {
                        return;
                    }
                    cx.draw_pass(*pass_id, &mut self.metal_cx, DrawPassMode::Drawable(drawable));
                }
                CxPassParent::Pass(_) | CxPassParent::None => {
                    cx.draw_pass(*pass_id, &mut self.metal_cx, DrawPassMode::Texture);
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
        unsafe {
            let () = msg_send![self.host_view, setLayer: nil];
            let () = msg_send![self.ca_layer, release];
        }
    }
}
