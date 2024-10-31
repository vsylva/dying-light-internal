//! This module contains functions related to processing input events.

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};

use crate::renderer::{Pipeline, RenderEngine};

pub type WndProcType =
    unsafe extern "system" fn(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;

// Replication of the Win32 HIWORD macro.
#[inline]
pub fn hiword(l: u32) -> u16 {
    ((l >> 16) & 0xffff) as u16
}

// Replication of the Win32 LOWORD macro.
#[inline]
pub fn loword(l: u32) -> u16 {
    (l & 0xffff) as u16
}

pub fn imgui_wnd_proc_impl<T: RenderEngine>(
    hwnd: HWND,
    umsg: u32,
    WPARAM(wparam): WPARAM,
    LPARAM(lparam): LPARAM,
    pipeline: &mut Pipeline<T>,
) {
    let io = pipeline.context().io_mut();

    match umsg {
        WM_CHAR => io.add_input_character(char::from_u32(wparam as u32).unwrap()),
        WM_SIZE => {
            pipeline.resize(loword(lparam as u32) as u32, hiword(lparam as u32) as u32);
        }
        _ => {}
    };

    pipeline
        .render_loop()
        .on_wnd_proc(hwnd, umsg, WPARAM(wparam), LPARAM(lparam));
}
