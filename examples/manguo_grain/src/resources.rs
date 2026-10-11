//! Locate the plugin's bundled resources (fonts, icons) at runtime.
//!
//! Makepad resolves `dep("crate://<crate>/resources/..")` against the
//! crate's manifest directory, which only exists on the machine that built
//! the plugin. For a distributed bundle we point the live registry's
//! `package_root` at a folder shipped next to the binary, laid out as
//! `<root>/<crate_name>/resources/...`:
//!
//! - macOS `.vst3` / `.clap` bundles: `Contents/Resources/makepad`
//! - Windows `.vst3` bundle: `Contents/Resources/makepad`
//!   (binary in `Contents/x86_64-win/`)
//! - Windows `.clap`: a `Resources/makepad` folder next to the `.clap` file
//!
//! `MANGUO_GRAIN_RESOURCES=<root>` overrides the lookup. When nothing is
//! found (development checkout), the manifest paths are left in place.

use crate::makepad_widgets::Cx;
use std::path::{Path, PathBuf};

/// The resource root to use, if the plugin runs from a bundle.
pub fn locate() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MANGUO_GRAIN_RESOURCES") {
        let p = PathBuf::from(dir);
        if is_root(&p) {
            return Some(p);
        }
    }
    let module = module_path()?;
    let dir = module.parent()?;
    let candidates = [
        dir.join("Resources").join("makepad"),
        dir.join("..").join("Resources").join("makepad"),
    ];
    candidates.into_iter().find(|p| is_root(p)).and_then(|p| p.canonicalize().ok())
}

fn is_root(p: &Path) -> bool {
    p.join("manguo_grain").join("resources").is_dir()
}

/// Point makepad at `root`. Call before `init_cx_os`.
pub fn apply(cx: &mut Cx, root: &Path) {
    let root = root.to_string_lossy().replace('\\', "/");
    cx.live_registry.borrow_mut().package_root = Some(root);
}

#[cfg(target_os = "macos")]
fn module_path() -> Option<PathBuf> {
    use std::ffi::{c_char, c_int, c_void, CStr};
    #[repr(C)]
    struct DlInfo {
        fname: *const c_char,
        fbase: *mut c_void,
        sname: *const c_char,
        saddr: *mut c_void,
    }
    extern "C" {
        fn dladdr(addr: *const c_void, info: *mut DlInfo) -> c_int;
    }
    let mut info = DlInfo { fname: std::ptr::null(), fbase: std::ptr::null_mut(), sname: std::ptr::null(), saddr: std::ptr::null_mut() };
    let ok = unsafe { dladdr(module_path as *const c_void, &mut info) };
    if ok == 0 || info.fname.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(info.fname) }.to_string_lossy().into_owned();
    Some(PathBuf::from(s))
}

#[cfg(target_os = "windows")]
fn module_path() -> Option<PathBuf> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStringExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleExW(flags: u32, name: *const u16, out: *mut *mut c_void) -> i32;
        fn GetModuleFileNameW(module: *mut c_void, buf: *mut u16, len: u32) -> u32;
    }
    const FROM_ADDRESS: u32 = 0x4;
    const UNCHANGED_REFCOUNT: u32 = 0x2;
    let mut module: *mut c_void = std::ptr::null_mut();
    let ok = unsafe { GetModuleHandleExW(FROM_ADDRESS | UNCHANGED_REFCOUNT, module_path as *const u16, &mut module) };
    if ok == 0 {
        return None;
    }
    let mut buf = vec![0u16; 32768];
    let n = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 || n >= buf.len() {
        return None;
    }
    Some(PathBuf::from(std::ffi::OsString::from_wide(&buf[..n])))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn module_path() -> Option<PathBuf> {
    None
}
