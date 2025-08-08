#![allow(static_mut_refs)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

use imgui::{Context, TextureId};
use once_cell::sync::OnceCell;

pub use imgui;
pub use windows;
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, WPARAM},
        System::{
            Console::{
                AllocConsole, CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING, FreeConsole,
                GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE, SetConsoleMode,
            },
            LibraryLoader::FreeLibraryAndExitThread,
        },
    },
    core::Error,
};

use crate::mh::{MH_ApplyQueued, MH_Initialize, MH_STATUS, MH_Uninitialize, MhHook};

pub mod hooks;
pub mod mh;
pub(crate) mod renderer;

pub mod util;

static mut MODULE: OnceCell<HINSTANCE> = OnceCell::new();
static mut HUDHOOK: OnceCell<Hudhook> = OnceCell::new();
static CONSOLE_ALLOCATED: AtomicBool = AtomicBool::new(false);

pub trait RenderContext {
    fn load_texture(&mut self, data: &[u8], width: u32, height: u32) -> Result<TextureId, Error>;

    fn replace_texture(
        &mut self,
        texture_id: TextureId,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Error>;
}

pub fn alloc_console() -> Result<(), Error> {
    if !CONSOLE_ALLOCATED.swap(true, Ordering::SeqCst) {
        unsafe { AllocConsole()? };
    }

    Ok(())
}

pub fn enable_console_colors() {
    if CONSOLE_ALLOCATED.load(Ordering::SeqCst) {
        unsafe {
            let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE).unwrap();

            let mut current_console_mode = CONSOLE_MODE(0);
            GetConsoleMode(stdout_handle, &mut current_console_mode).unwrap();

            current_console_mode.0 |= ENABLE_VIRTUAL_TERMINAL_PROCESSING.0;

            SetConsoleMode(stdout_handle, current_console_mode).unwrap();
        }
    }
}

pub fn free_console() -> Result<(), Error> {
    if CONSOLE_ALLOCATED.swap(false, Ordering::SeqCst) {
        unsafe { FreeConsole()? };
    }

    Ok(())
}

pub fn eject() {
    thread::spawn(|| unsafe {
        free_console().unwrap_or_default();

        if let Some(mut hudhook) = HUDHOOK.take() {
            hudhook.unapply().unwrap_or_default()
        }

        if let Some(module) = MODULE.take() {
            FreeLibraryAndExitThread(module, 0);
        }
    });
}

pub trait ImguiRenderLoop {
    unsafe fn initialize<'a>(
        &'a mut self,
        ctx: &mut Context,
        render_context: &'a mut dyn RenderContext,
    );

    unsafe fn render(&mut self, ctx: &mut Context);

    unsafe fn on_wnd_proc(&self, _hwnd: HWND, _umsg: u32, _wparam: WPARAM, _lparam: LPARAM) {}
}

pub trait Hooks {
    fn from_render_loop<T>(t: T) -> Box<Self>
    where
        Self: Sized,
        T: ImguiRenderLoop + Send + Sync + 'static;

    fn hooks(&self) -> &[MhHook];

    unsafe fn unhook(&mut self);
}

pub struct Hudhook(Vec<Box<dyn Hooks>>);
unsafe impl Send for Hudhook {}
unsafe impl Sync for Hudhook {}

impl Hudhook {
    pub fn builder() -> HudhookBuilder {
        HudhookBuilder(Hudhook::new())
    }

    fn new() -> Self {
        match unsafe { MH_Initialize() } {
            MH_STATUS::MH_ERROR_ALREADY_INITIALIZED | MH_STATUS::MH_OK => {}
            status @ MH_STATUS::MH_ERROR_MEMORY_ALLOC => panic!("MH_Initialize: {status:?}"),
            _ => unreachable!(),
        }

        Hudhook(Vec::new())
    }

    fn hooks(&self) -> impl IntoIterator<Item = &MhHook> {
        self.0.iter().flat_map(|h| h.hooks())
    }

    pub fn apply(self) -> Result<(), MH_STATUS> {
        for hook in self.hooks() {
            unsafe { hook.queue_enable()? };
        }

        unsafe { MH_ApplyQueued().ok_context()? };

        unsafe { HUDHOOK.set(self).ok() };

        Ok(())
    }

    pub fn unapply(&mut self) -> Result<(), MH_STATUS> {
        for hook in self.hooks() {
            unsafe { hook.queue_disable()? };
        }

        unsafe { MH_ApplyQueued().ok_context()? };

        unsafe { MH_Uninitialize().ok_context()? };

        for hook in &mut self.0 {
            unsafe { hook.unhook() };
        }

        Ok(())
    }
}

pub struct HudhookBuilder(Hudhook);

impl HudhookBuilder {
    pub fn with<T: Hooks + 'static>(
        mut self,
        render_loop: impl ImguiRenderLoop + Send + Sync + 'static,
    ) -> Self {
        self.0.0.push(T::from_render_loop(render_loop));
        self
    }

    pub fn with_hmodule(self, module: HINSTANCE) -> Self {
        unsafe { MODULE.set(module).unwrap() };
        self
    }

    pub fn build(self) -> Hudhook {
        self.0
    }
}

#[macro_export]
macro_rules! hudhook {
    ($t:ty, $hooks:expr) => {
        #[no_mangle]
        pub unsafe extern "stdcall" fn DllMain(
            hmodule: ::hudhook::windows::Win32::Foundation::HINSTANCE,
            reason: u32,
            _: *mut ::std::ffi::c_void,
        ) {
            use ::hudhook::*;

            if reason == ::hudhook::windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH {
                ::hudhook::tracing::trace!("DllMain()");
                ::std::thread::spawn(move || {
                    if let Err(e) = ::hudhook::Hudhook::builder()
                        .with::<$t>({ $hooks })
                        .with_hmodule(hmodule)
                        .build()
                        .apply()
                    {
                        ::hudhook::tracing::error!("Couldn't apply hooks: {e:?}");
                        ::hudhook::eject();
                    }
                });
            }
        }
    };
}
