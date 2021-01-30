#![cfg(windows)]
#![allow(non_snake_case)]

use std::marker::PhantomData;

use winapi::shared::windef::*;
use winapi::um::winuser::*;

use std::mem::zeroed;
use std::ptr::*;



pub struct Window<'w> {
    hwnd:       HWND,
    phantom:    PhantomData<&'w HWND>,
}

impl Window<'_> {
    pub unsafe fn from_raw(hwnd: HWND) -> Self {
        debug_assert!(unsafe { IsWindow(hwnd) } != 0, "not a valid HWND");
        Self { hwnd, phantom: PhantomData }
    }

    pub fn as_hwnd(&self) -> HWND { self.hwnd }
}

impl Drop for Window<'_> {
    fn drop(&mut self) {
        debug_assert!(unsafe { IsWindow(self.hwnd) } != 0, "HWND destroyed before Window was dropped");
    }
}



pub(crate) fn run() {
    loop {
        let mut msg = unsafe { zeroed::<MSG>() };
        while unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            match msg.message {
                WM_QUIT => return,
                _other  => {},
            }
            unsafe { TranslateMessage(&msg) };
            unsafe { DispatchMessageW(&msg) };
        }

        crate::update::run_local();
        crate::d3d9::render_all();
        crate::d3d9::swap_all();
    }
}
