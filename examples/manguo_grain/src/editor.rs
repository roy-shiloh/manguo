//! Host editor window: baseview owns the child NSView and the event loop,
//! makepad renders into it through `EmbeddedCx` and receives the converted
//! events. Everything here runs on the host's UI thread.

use crate::app::{App, PluginUi, EDITOR_HEIGHT, EDITOR_WIDTH};
use crate::grain_core::GridShared;
#[cfg(target_os = "macos")]
use crate::makepad_widgets::makepad_platform::os::apple::macos::EmbeddedCx;
#[cfg(target_os = "windows")]
use crate::makepad_widgets::makepad_platform::os::windows::EmbeddedCx;
use crate::makepad_widgets::*;
use crate::GrainParams;
use baseview::{HandlerError, WindowContext, WindowSettings};
use nice_plug::context::gui::GuiContext;
use nice_plug::editor::dpi::{LogicalSize, NativeSize};
use nice_plug::editor::{Editor, EditorHandle, HostMethods, ParentWindowHandle, ResizeHint, SpawnedEditor};
use nice_plug::prelude::*;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const RESIZE_HINT: ResizeHint = ResizeHint::non_resizable();

pub struct GrainEditor {
    params: Arc<GrainParams>,
    shared: Arc<GridShared>,
    open: Arc<AtomicBool>,
}

impl GrainEditor {
    pub fn new(params: Arc<GrainParams>, shared: Arc<GridShared>) -> Self {
        Self { params, shared, open: Arc::new(AtomicBool::new(false)) }
    }
}

/// Everything the baseview handler owns. `WindowHandler` methods take `&self`,
/// so the makepad side lives behind a `RefCell`. Public so automated hosts
/// can build the window and drive `inject` for input.
pub struct GrainWindow {
    window: WindowContext,
    inner: RefCell<GrainWindowInner>,
}

struct GrainWindowInner {
    embedded: EmbeddedCx,
    cursor: DVec2,
    modifiers: KeyModifiers,
    scale: f64,
}

impl GrainWindow {
    pub fn new(window: WindowContext, plug: PluginUi) -> Result<Self, HandlerError> {
        let raw = window.window_handle().map(|h| h.as_raw());
        let size = window.size();
        let scale = size.scale_factor;

        let plug = Rc::new(plug);
        let app: Rc<RefCell<Option<App>>> = Rc::new(RefCell::new(None));
        let app_for_cx = app.clone();
        let plug_for_cx = plug.clone();
        let cx = Rc::new(RefCell::new(Cx::new(Box::new(move |cx, event| {
            if let Event::Startup = event {
                let mut new_app = App::new_main(cx);
                if let Some(a) = new_app.as_mut() {
                    a.plug = Some(plug_for_cx.clone());
                }
                *app_for_cx.borrow_mut() = new_app;
            }
            if let Event::LiveEdit = event {
                app_for_cx.borrow_mut().update_main(cx);
            }
            if let Some(app) = &mut *app_for_cx.borrow_mut() {
                <dyn AppMain>::handle_event(app, cx, event);
            }
        }))));
        {
            let mut c = cx.borrow_mut();
            App::register_main_module(&mut c);
            crate::app::live_design(&mut c);
            if let Some(root) = crate::resources::locate() {
                crate::resources::apply(&mut c, &root);
            }
            c.init_cx_os();
        }
        let logical = dvec2(size.logical.width, size.logical.height);
        #[cfg(target_os = "macos")]
        let embedded = match raw {
            Ok(RawWindowHandle::AppKit(h)) => EmbeddedCx::new(cx, h.ns_view.as_ptr() as *mut std::ffi::c_void, logical, scale),
            _ => return Err(HandlerError::from_boxed("manguo_grain: host did not give an AppKit NSView".into())),
        };
        #[cfg(target_os = "windows")]
        let embedded = match raw {
            Ok(RawWindowHandle::Win32(h)) => EmbeddedCx::new(cx, h.hwnd.get(), logical, scale),
            _ => return Err(HandlerError::from_boxed("manguo_grain: host did not give a Win32 HWND".into())),
        };
        Ok(Self {
            window,
            inner: RefCell::new(GrainWindowInner {
                embedded,
                cursor: dvec2(0.0, 0.0),
                modifiers: KeyModifiers::default(),
                scale,
            }),
        })
    }
}

impl GrainWindow {
    /// Direct access to the embedded context, e.g. to inject synthetic input.
    pub fn inject(&self, f: impl FnOnce(&mut EmbeddedCx)) {
        f(&mut self.inner.borrow_mut().embedded);
    }
}

fn convert_modifiers(m: keyboard_types::Modifiers) -> KeyModifiers {
    KeyModifiers {
        shift: m.contains(keyboard_types::Modifiers::SHIFT),
        control: m.contains(keyboard_types::Modifiers::CONTROL),
        alt: m.contains(keyboard_types::Modifiers::ALT),
        logo: m.contains(keyboard_types::Modifiers::META),
    }
}

fn convert_button(b: baseview::MouseButton) -> MouseButton {
    match b {
        baseview::MouseButton::Left => MouseButton::PRIMARY,
        baseview::MouseButton::Right => MouseButton::SECONDARY,
        baseview::MouseButton::Middle => MouseButton::MIDDLE,
        baseview::MouseButton::Back => MouseButton::BACK,
        baseview::MouseButton::Forward => MouseButton::FORWARD,
        _ => MouseButton::empty(),
    }
}

fn convert_key(key: &keyboard_types::Key) -> KeyCode {
    use keyboard_types::{Key, NamedKey};
    match key {
        Key::Named(n) => match n {
            NamedKey::Escape => KeyCode::Escape,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::Enter => KeyCode::ReturnKey,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::ArrowUp => KeyCode::ArrowUp,
            NamedKey::ArrowDown => KeyCode::ArrowDown,
            NamedKey::ArrowLeft => KeyCode::ArrowLeft,
            NamedKey::ArrowRight => KeyCode::ArrowRight,
            NamedKey::Home => KeyCode::Home,
            NamedKey::End => KeyCode::End,
            NamedKey::PageUp => KeyCode::PageUp,
            NamedKey::PageDown => KeyCode::PageDown,
            NamedKey::Shift => KeyCode::Shift,
            NamedKey::Control => KeyCode::Control,
            NamedKey::Alt => KeyCode::Alt,
            NamedKey::Meta => KeyCode::Logo,
            _ => KeyCode::Unknown,
        },
        Key::Character(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => match c.to_ascii_lowercase() {
                    ' ' => KeyCode::Space,
                    '-' => KeyCode::Minus,
                    '=' => KeyCode::Equals,
                    '.' => KeyCode::Period,
                    '0' => KeyCode::Key0,
                    '1' => KeyCode::Key1,
                    '2' => KeyCode::Key2,
                    '3' => KeyCode::Key3,
                    '4' => KeyCode::Key4,
                    '5' => KeyCode::Key5,
                    '6' => KeyCode::Key6,
                    '7' => KeyCode::Key7,
                    '8' => KeyCode::Key8,
                    '9' => KeyCode::Key9,
                    'a' => KeyCode::KeyA,
                    'b' => KeyCode::KeyB,
                    'c' => KeyCode::KeyC,
                    'd' => KeyCode::KeyD,
                    'e' => KeyCode::KeyE,
                    'f' => KeyCode::KeyF,
                    'g' => KeyCode::KeyG,
                    'h' => KeyCode::KeyH,
                    'i' => KeyCode::KeyI,
                    'j' => KeyCode::KeyJ,
                    'k' => KeyCode::KeyK,
                    'l' => KeyCode::KeyL,
                    'm' => KeyCode::KeyM,
                    'n' => KeyCode::KeyN,
                    'o' => KeyCode::KeyO,
                    'p' => KeyCode::KeyP,
                    'q' => KeyCode::KeyQ,
                    'r' => KeyCode::KeyR,
                    's' => KeyCode::KeyS,
                    't' => KeyCode::KeyT,
                    'u' => KeyCode::KeyU,
                    'v' => KeyCode::KeyV,
                    'w' => KeyCode::KeyW,
                    'x' => KeyCode::KeyX,
                    'y' => KeyCode::KeyY,
                    'z' => KeyCode::KeyZ,
                    _ => KeyCode::Unknown,
                },
                _ => KeyCode::Unknown,
            }
        }
    }
}

impl baseview::WindowHandler for GrainWindow {
    fn on_frame(&self) -> Result<(), HandlerError> {
        self.inner.borrow_mut().embedded.frame();
        Ok(())
    }

    fn resized(&self, new_size: baseview::WindowSize) -> Result<(), HandlerError> {
        let mut inner = self.inner.borrow_mut();
        inner.scale = new_size.scale_factor;
        inner
            .embedded
            .resize(dvec2(new_size.logical.width, new_size.logical.height), new_size.scale_factor);
        Ok(())
    }

    fn on_event(&self, event: baseview::Event) -> baseview::EventStatus {
        use baseview::{Event as BvEvent, EventStatus, MouseEvent, ScrollDelta, WindowEvent};
        let mut inner = self.inner.borrow_mut();
        let inner = &mut *inner;
        match event {
            BvEvent::Mouse(m) => {
                match m {
                    MouseEvent::CursorMoved { position, modifiers } => {
                        inner.cursor = dvec2(position.x / inner.scale, position.y / inner.scale);
                        inner.modifiers = convert_modifiers(modifiers);
                        inner.embedded.mouse_move(inner.cursor, inner.modifiers);
                    }
                    MouseEvent::ButtonPressed { button, modifiers } => {
                        inner.modifiers = convert_modifiers(modifiers);
                        if !self.window.has_focus() {
                            let _ = self.window.focus();
                        }
                        inner.embedded.mouse_down(inner.cursor, convert_button(button), inner.modifiers);
                    }
                    MouseEvent::ButtonReleased { button, modifiers } => {
                        inner.modifiers = convert_modifiers(modifiers);
                        inner.embedded.mouse_up(inner.cursor, convert_button(button), inner.modifiers);
                    }
                    MouseEvent::WheelScrolled { delta, modifiers } => {
                        inner.modifiers = convert_modifiers(modifiers);
                        let (scroll, is_mouse) = match delta {
                            ScrollDelta::Pixels { x, y } => (dvec2(-x as f64, -y as f64), false),
                            ScrollDelta::Lines { x, y } => (dvec2(-x as f64 * 32.0, -y as f64 * 32.0), true),
                        };
                        inner.embedded.scroll(inner.cursor, scroll, is_mouse, inner.modifiers);
                    }
                    MouseEvent::CursorLeft => {
                        inner.embedded.mouse_move(dvec2(-1.0, -1.0), inner.modifiers);
                    }
                    _ => {}
                }
                EventStatus::Captured
            }
            BvEvent::Keyboard(k) => {
                let mods = convert_modifiers(k.modifiers);
                inner.modifiers = mods;
                let code = convert_key(&k.key);
                match k.state {
                    keyboard_types::KeyState::Down => {
                        inner.embedded.key_down(code, k.repeat, mods);
                        if let keyboard_types::Key::Character(s) = &k.key {
                            if !mods.control && !mods.logo && !k.is_composing {
                                inner.embedded.text_input(s.clone());
                            }
                        }
                    }
                    keyboard_types::KeyState::Up => inner.embedded.key_up(code, mods),
                }
                // let the host keep its shortcuts when the UI did not want the key
                if code == KeyCode::Unknown && mods.logo {
                    EventStatus::Ignored
                } else {
                    EventStatus::Captured
                }
            }
            BvEvent::Window(w) => {
                match w {
                    WindowEvent::Focused => inner.embedded.focus(true),
                    WindowEvent::Unfocused => inner.embedded.focus(false),
                    _ => {}
                }
                EventStatus::Ignored
            }
            _ => EventStatus::Ignored,
        }
    }
}

impl Editor for GrainEditor {
    type Handle = GrainEditorHandle;

    fn spawn(
        &self,
        parent: Option<ParentWindowHandle>,
        wait_for_parent: bool,
        fallback_scale_factor: Option<f64>,
        gui_context: GuiContext,
        host: Option<HostMethods>,
    ) -> Result<SpawnedEditor<Self::Handle>, Box<dyn Error>> {
        let params = self.params.clone();
        let shared = self.shared.clone();

        let host = {
            struct HostCallbackAdapter {
                host: Box<dyn nice_plug::editor::HostCallbacks>,
            }
            impl baseview::host::HostCallbacks for HostCallbackAdapter {
                fn request_resize(&mut self, new_size: baseview::WindowSize) -> Result<(), HandlerError> {
                    self.host
                        .request_resize(new_size.physical.into(), new_size.scale_factor)
                        .map_err(HandlerError::from_boxed)
                }
                fn destroyed(&mut self) {
                    self.host.destroyed();
                }
            }
            struct HostMainThreadCallerAdapter {
                host: Box<dyn nice_plug::editor::HostMainThreadCaller>,
            }
            impl baseview::host::HostMainThreadCaller for HostMainThreadCallerAdapter {
                fn call_main_thread(&mut self) {
                    self.host.call_main_thread();
                }
            }
            host.map(|host| {
                baseview::host::Host::new()
                    .with_callbacks(HostCallbackAdapter { host: host.callbacks })
                    .with_main_thread(HostMainThreadCallerAdapter { host: host.main_thread_caller })
            })
        };

        let window = baseview::Window::create_with_host(
            WindowSettings::new()
                .with_title("Manguo Grain")
                .with_size(LogicalSize::new(EDITOR_WIDTH as f32, EDITOR_HEIGHT as f32))
                .with_resizable(RESIZE_HINT.can_resize)
                .with_parent(parent.as_ref())
                .with_wait_for_parent(wait_for_parent)
                .with_fallback_scale_factor(fallback_scale_factor),
            move |window: WindowContext| -> Result<GrainWindow, HandlerError> {
                GrainWindow::new(window, PluginUi { params, shared, gui: gui_context })
            },
            host,
        )?;

        self.open.store(true, Ordering::Release);
        Ok(SpawnedEditor { handle: GrainEditorHandle { open: self.open.clone() }, window })
    }

    fn size(&self) -> NativeSize<u32> {
        NativeSize::new(EDITOR_WIDTH as u32, EDITOR_HEIGHT as u32)
    }

    fn resize_hint(&self) -> ResizeHint {
        RESIZE_HINT
    }

    unsafe fn assume_standalone_in_process(&self) {
        unsafe { baseview::assume_standalone_in_process() }
    }
}

pub struct GrainEditorHandle {
    open: Arc<AtomicBool>,
}

impl EditorHandle for GrainEditorHandle {
    type Window = baseview::Window;
    type Error = baseview::Error;

    fn run_until_closed(window: Self::Window) -> Result<(), Self::Error> {
        window.run_until_closed()
    }

    fn set_parent(&self, parent: ParentWindowHandle, window: &Self::Window) -> Result<(), Self::Error> {
        window.set_parent(&parent)
    }

    fn show(&self, window: &Self::Window) -> Result<(), Self::Error> {
        window.show()
    }

    fn hide(&self, window: &Self::Window) -> Result<(), Self::Error> {
        window.hide()
    }

    fn host_main_thread_callback(&self, window: &Self::Window) {
        window.host_main_thread_callback();
    }

    fn set_size(&self, new_size: NativeSize<u32>, window: &Self::Window) -> Result<(), Self::Error> {
        window.resize(new_size)
    }

    fn set_fallback_scale_factor(&self, scale_factor: f64, window: &Self::Window) -> Result<(), Self::Error> {
        window.suggest_fallback_scale_factor(scale_factor)
    }

    fn adjust_size(&self, _new_size: NativeSize<u32>, _window: &Self::Window) -> Option<NativeSize<u32>> {
        Some(NativeSize::new(EDITOR_WIDTH as u32, EDITOR_HEIGHT as u32))
    }

    fn on_virtual_key_from_host(&self, _key_code: VirtualKeyCode, _is_down: bool, _modifiers: Modifiers) -> bool {
        false
    }

    fn state_changed(&self) {}

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
}

impl Drop for GrainEditorHandle {
    fn drop(&mut self) {
        self.open.store(false, Ordering::Release);
    }
}
