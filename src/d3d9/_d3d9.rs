#![cfg(windows)]
#![allow(non_snake_case)]

use crate::windows::Window;

use wchar::wch_c;

use winapi::shared::d3d9::*;
use winapi::shared::d3d9types::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::winerror::*;

use winapi::um::errhandlingapi::*;
use winapi::um::libloaderapi::*;
use winapi::um::winuser::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::Display;
use std::mem::zeroed;
use std::os::windows::ffi::*;
use std::ptr::*;
use std::sync::Once;

const WIDTH     : u16 = 800;
const HEIGHT    : u16 = 600;

//TODO: allow resizing up to a maximium size
//const STYLE     : u32 = WS_OVERLAPPEDWINDOW;
const STYLE     : u32 = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU /*| WS_THICKFRAME*/ | WS_MINIMIZEBOX /* | WS_MAXIMIZEBOX*/;



pub struct CreateContext {
    pub device:         mcom::Rc<IDirect3DDevice9>,
    _non_exhaustive:    (),
}

type CreateFn = Box<dyn Fn(&CreateContext) -> Box<dyn Layer>>;



pub struct RenderContext {
    pub device:         mcom::Rc<IDirect3DDevice9>,
    _non_exhaustive:    (),
}

pub trait Layer : 'static {
    #[allow(unused_variables)]
    fn render(&self, ctx: &mut RenderContext) { }
}

impl<L: Layer> Layer for Vec<L> {
    fn render(&self, ctx: &mut RenderContext) {
        for layer in self.iter().rev() { layer.render(ctx); }
    }
}

#[derive(Default)]
pub struct CreateWindowParameters {
    pub title:          OsString,
    _non_exhaustive:    (),
}

impl From<()>       for CreateWindowParameters { fn from(_: ()) -> CreateWindowParameters { Default::default() } }
impl From<OsString> for CreateWindowParameters { fn from(title: OsString) -> CreateWindowParameters { CreateWindowParameters { title, ..Default::default() } } }
impl From<String>   for CreateWindowParameters { fn from(title: String  ) -> CreateWindowParameters { CreateWindowParameters { title: title.into(), ..Default::default() } } }
impl From<&str>     for CreateWindowParameters { fn from(title: &str    ) -> CreateWindowParameters { CreateWindowParameters { title: title.into(), ..Default::default() } } }

pub fn create_window<L: Layer>(params: impl Into<CreateWindowParameters>, root_fn: impl Fn(&CreateContext) -> L + 'static) {
    ensure_init();

    let params = params.into();
    let title = params.title.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let root_fn : Box<CreateFn> = Box::new(Box::new(move |cc| Box::new(root_fn(cc)))); // inner box = fat pointer, outer box = winapi compatible thin pointer

    let hInstance = unsafe { GetModuleHandleW(null_mut()) };

    let hwnd = unsafe { CreateWindowExW(
        0, wch_c!("ForecastleD3D9").as_ptr(), title.as_ptr(), STYLE | WS_VISIBLE,
        CW_USEDEFAULT, CW_USEDEFAULT, WIDTH.into(), HEIGHT.into(),
        null_mut(), null_mut(), hInstance, Box::into_raw(root_fn).cast(),
    )};
    assert!(!hwnd.is_null());
}



pub(crate) fn render_all() {
    TL.with(|tl|{
        for pw in tl.windows.borrow().values() {
            pw.root.render(&mut RenderContext {
                device: pw.device.clone(),
                _non_exhaustive: (),
            });
        }
    })
}

pub(crate) fn swap_all() {
    TL.with(|tl|{
        for pw in tl.windows.borrow().values() {
            // TODO: reset/lost handling
            let _hr = unsafe { pw.device.Present(null_mut(), null_mut(), null_mut(), null_mut()) };
            assert!(SUCCEEDED(_hr), "device reset / lost handling not yet implemented");
        }
    })
}



unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => TL.with(|tl|{
            let cs = &*(lparam as *const CREATESTRUCTW);
            let root_fn = Box::from_raw(cs.lpCreateParams as *mut CreateFn);
            let mut windows = tl.windows.borrow_mut();
            let prev = windows.insert(hwnd, PerWindow::new(&tl.common, root_fn, Window::from_raw(hwnd)));
            assert!(prev.is_none(), "WM_CREATE for previously registered window");
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }),
        WM_DESTROY => TL.with(|tl|{
            let mut windows = tl.windows.borrow_mut();
            let prev = windows.remove(&hwnd);
            assert!(prev.is_some(), "WM_DESTROY for unregistered window");
            if windows.is_empty() {
                PostQuitMessage(0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }),
        _other => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}



thread_local! {
    static TL : ThreadLocal = ThreadLocal::default();
}

#[derive(Default)]
struct ThreadLocal {
    common:     Common,
    windows:    RefCell<HashMap<HWND, PerWindow>>,
}



struct Common {
    d3d9:   mcom::Rc<IDirect3D9>,
}

impl Default for Common {
    fn default() -> Self {
        const D3D_SDK_DEBUG : u32 = 0x80000000;

        let mut d3d9 = unsafe { Direct3DCreate9(D3D_SDK_VERSION | D3D_SDK_DEBUG) };
        if d3d9.is_null() { d3d9 = unsafe { Direct3DCreate9(D3D_SDK_VERSION &! D3D_SDK_DEBUG) }; }
        let d3d9 = unsafe { mcom::Rc::from_raw_opt(d3d9) }.unwrap_or_else(|| panic!("Direct3DCreate9 failed with {}", gle()));

        Self { d3d9 }
    }
}



struct PerWindow {
    device: mcom::Rc<IDirect3DDevice9>,
    root:   Box<dyn Layer>,
}

impl PerWindow {
    pub fn new(common: &Common, root_fn: Box<CreateFn>, window: Window) -> Self {
        let hwnd = window.as_hwnd();

        let mut device = null_mut();
        let flags = D3DCREATE_FPU_PRESERVE | D3DCREATE_HARDWARE_VERTEXPROCESSING;
        let hr = unsafe { common.d3d9.CreateDevice(0, D3DDEVTYPE_HAL, hwnd, flags, &mut D3DPRESENT_PARAMETERS {
            hDeviceWindow:  hwnd,
            Windowed:       1,
            SwapEffect:     D3DSWAPEFFECT_DISCARD,
            .. zeroed()
        }, &mut device) };
        assert!(SUCCEEDED(hr), "IDirect3D9::CreateDevice failed with {}", hre(hr));
        let device = unsafe { mcom::Rc::from_raw(device) };

        let root = root_fn(&CreateContext {
            device: device.clone(),
            _non_exhaustive: (),
        });

        Self { device, root }
    }
}



fn ensure_init() {
    static ONCE : Once = Once::new();
    ONCE.call_once(||{
        let hInstance = unsafe { GetModuleHandleW(null_mut()) };

        let atom = unsafe { RegisterClassW(&WNDCLASSW {
            hCursor:            LoadCursorW(null_mut(), IDC_ARROW),
            hInstance,
            lpfnWndProc:        Some(wndproc),
            lpszClassName:      wch_c!("ForecastleD3D9").as_ptr(),
            .. zeroed()
        })};
        assert!(atom != 0, "RegisterClassW failed");
    })
}

fn gle() -> impl Display {
    let e = unsafe { GetLastError() };
    format!("GetLastError() == 0x{:08x}", e)
}

fn hre(hr: HRESULT) -> impl Display {
    format!("HRESULT == 0x{:08x}", hr as u32)
}
