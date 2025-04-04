//! Hooks for DirectX 12.

use std::{ffi::c_void, mem, sync::OnceLock};

use imgui::Context;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use windows::{
    Win32::{
        Foundation::BOOL,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
                D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12CreateDevice, ID3D12CommandList,
                ID3D12CommandQueue, ID3D12Device, ID3D12Resource,
            },
            Dxgi::{
                Common::{
                    DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC,
                    DXGI_MODE_SCALING_UNSPECIFIED, DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                    DXGI_RATIONAL, DXGI_SAMPLE_DESC,
                },
                CreateDXGIFactory2, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH,
                DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory2,
                IDXGISwapChain, IDXGISwapChain3,
            },
        },
    },
    core::{Error, HRESULT, Interface, Result},
};

use super::DummyHwnd;
use crate::{
    Hooks, ImguiRenderLoop,
    mh::MhHook,
    renderer::{D3D12RenderEngine, Pipeline},
    util,
};

type DXGISwapChainPresentType =
    unsafe extern "system" fn(This: IDXGISwapChain3, SyncInterval: u32, Flags: u32) -> HRESULT;

type DXGISwapChainResizeBuffersType = unsafe extern "system" fn(
    This: IDXGISwapChain3,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    flags: u32,
) -> HRESULT;

type D3D12CommandQueueExecuteCommandListsType = unsafe extern "system" fn(
    This: ID3D12CommandQueue,
    num_command_lists: u32,
    command_lists: *mut ID3D12CommandList,
);

struct Trampolines {
    dxgi_swap_chain_present: DXGISwapChainPresentType,
    dxgi_swap_chain_resize_buffers: DXGISwapChainResizeBuffersType,
    d3d12_command_queue_execute_command_lists: D3D12CommandQueueExecuteCommandListsType,
}

static mut TRAMPOLINES: OnceLock<Trampolines> = OnceLock::new();

enum InitializationContext {
    Empty,
    WithSwapChain(IDXGISwapChain3),
    Complete(IDXGISwapChain3, ID3D12CommandQueue),
    Done,
}

impl InitializationContext {
    // Transition to a state where the swap chain is set. Ignore other mutations.
    fn insert_swap_chain(&mut self, swap_chain: &IDXGISwapChain3) {
        *self = match mem::replace(self, InitializationContext::Empty) {
            InitializationContext::Empty => {
                InitializationContext::WithSwapChain(swap_chain.clone())
            }
            s => s,
        }
    }

    // Transition to a complete state if the swap chain is set and the command queue
    // is associated with it.
    fn insert_command_queue(&mut self, command_queue: &ID3D12CommandQueue) {
        *self = match mem::replace(self, InitializationContext::Empty) {
            InitializationContext::WithSwapChain(swap_chain) => {
                if unsafe { Self::check_command_queue(&swap_chain, command_queue) } {
                    InitializationContext::Complete(swap_chain, command_queue.clone())
                } else {
                    InitializationContext::WithSwapChain(swap_chain)
                }
            }
            s => s,
        }
    }

    // Retrieve the values if the context is complete.
    fn get(&self) -> Option<(IDXGISwapChain3, ID3D12CommandQueue)> {
        if let InitializationContext::Complete(swap_chain, command_queue) = self {
            Some((swap_chain.clone(), command_queue.clone()))
        } else {
            None
        }
    }

    // Mark the context as done so no further operations are executed on it.
    fn done(&mut self) {
        if let InitializationContext::Complete(..) = self {
            *self = InitializationContext::Done;
        }
    }

    unsafe fn check_command_queue(
        swap_chain: &IDXGISwapChain3,
        command_queue: &ID3D12CommandQueue,
    ) -> bool {
        let swap_chain_ptr = swap_chain.as_raw() as *mut *mut c_void;
        let readable_ptrs = util::readable_region(swap_chain_ptr, 512);

        match readable_ptrs
            .iter()
            .position(|&ptr| ptr == command_queue.as_raw())
        {
            Some(_) => true,
            None => false,
        }
    }
}

static INITIALIZATION_CONTEXT: Mutex<InitializationContext> =
    Mutex::new(InitializationContext::Empty);
static mut PIPELINE: OnceCell<Mutex<Pipeline<D3D12RenderEngine>>> = OnceCell::new();
static mut RENDER_LOOP: OnceCell<Box<dyn ImguiRenderLoop + Send + Sync>> = OnceCell::new();

unsafe fn init_pipeline() -> Result<Mutex<Pipeline<D3D12RenderEngine>>> {
    let Some((swap_chain, command_queue)) = ({ INITIALIZATION_CONTEXT.lock().get() }) else {
        return Err(Error::from_hresult(HRESULT(-1)));
    };

    let hwnd = util::try_out_param(|v| swap_chain.GetDesc(v)).map(|desc| desc.OutputWindow)?;

    let mut ctx = Context::create();
    let engine = D3D12RenderEngine::new(&command_queue, &mut ctx)?;

    let Some(render_loop) = RENDER_LOOP.take() else {
        return Err(Error::from_hresult(HRESULT(-1)));
    };

    let pipeline = Pipeline::new(hwnd, ctx, engine, render_loop).map_err(|(e, render_loop)| {
        RENDER_LOOP.get_or_init(move || render_loop);
        e
    })?;

    {
        INITIALIZATION_CONTEXT.lock().done();
    }

    Ok(Mutex::new(pipeline))
}

fn render(swap_chain: &IDXGISwapChain3) -> Result<()> {
    unsafe {
        let pipeline = PIPELINE.get_or_try_init(|| init_pipeline())?;

        let Some(mut pipeline) = pipeline.try_lock() else {
            return Err(Error::from_hresult(HRESULT(-1)));
        };

        pipeline.prepare_render()?;

        let target: ID3D12Resource =
            swap_chain.GetBuffer(swap_chain.GetCurrentBackBufferIndex())?;

        pipeline.render(target)?;
    }

    Ok(())
}

unsafe extern "system" fn dxgi_swap_chain_present_impl(
    swap_chain: IDXGISwapChain3,
    sync_interval: u32,
    flags: u32,
) -> HRESULT {
    {
        INITIALIZATION_CONTEXT.lock().insert_swap_chain(&swap_chain);
    }

    let Trampolines {
        dxgi_swap_chain_present,
        ..
    } = TRAMPOLINES
        .get()
        .expect("DirectX 12 trampolines uninitialized");
    render(&swap_chain).unwrap_or_default();
    dxgi_swap_chain_present(swap_chain, sync_interval, flags)
}

unsafe extern "system" fn dxgi_swap_chain_resize_buffers_impl(
    p_this: IDXGISwapChain3,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    flags: u32,
) -> HRESULT {
    let Trampolines {
        dxgi_swap_chain_resize_buffers,
        ..
    } = TRAMPOLINES
        .get()
        .expect("DirectX 12 trampolines uninitialized");

    dxgi_swap_chain_resize_buffers(p_this, buffer_count, width, height, new_format, flags)
}

unsafe extern "system" fn d3d12_command_queue_execute_command_lists_impl(
    command_queue: ID3D12CommandQueue,
    num_command_lists: u32,
    command_lists: *mut ID3D12CommandList,
) {
    {
        INITIALIZATION_CONTEXT
            .lock()
            .insert_command_queue(&command_queue);
    }

    let Trampolines {
        d3d12_command_queue_execute_command_lists,
        ..
    } = TRAMPOLINES
        .get()
        .expect("DirectX 12 trampolines uninitialized");

    d3d12_command_queue_execute_command_lists(command_queue, num_command_lists, command_lists);
}

fn get_target_addrs() -> (
    DXGISwapChainPresentType,
    DXGISwapChainResizeBuffersType,
    D3D12CommandQueueExecuteCommandListsType,
) {
    let dummy_hwnd = DummyHwnd::new();

    let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(0) }.unwrap();
    let adapter = unsafe { factory.EnumAdapters(0) }.unwrap();

    let device: ID3D12Device =
        util::try_out_ptr(|v| unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, v) })
            .expect("D3D12CreateDevice failed");

    let command_queue: ID3D12CommandQueue = unsafe {
        device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
            Priority: 0,
            Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
            NodeMask: 0,
        })
    }
    .unwrap();

    let swap_chain: IDXGISwapChain = match util::try_out_ptr(|v| unsafe {
        factory
            .CreateSwapChain(
                &command_queue,
                &DXGI_SWAP_CHAIN_DESC {
                    BufferDesc: DXGI_MODE_DESC {
                        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                        ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                        Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
                        Width: 640,
                        Height: 480,
                        RefreshRate: DXGI_RATIONAL {
                            Numerator: 60,
                            Denominator: 1,
                        },
                    },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    OutputWindow: dummy_hwnd.hwnd(),
                    Windowed: BOOL(1),
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as _,
                },
                v,
            )
            .ok()
    }) {
        Ok(swap_chain) => swap_chain,
        Err(e) => {
            panic!("{e:?}");
        }
    };

    let present_ptr: DXGISwapChainPresentType =
        unsafe { mem::transmute(swap_chain.vtable().Present) };
    let resize_buffers_ptr: DXGISwapChainResizeBuffersType =
        unsafe { mem::transmute(swap_chain.vtable().ResizeBuffers) };
    let cqecl_ptr: D3D12CommandQueueExecuteCommandListsType =
        unsafe { mem::transmute(command_queue.vtable().ExecuteCommandLists) };

    (present_ptr, resize_buffers_ptr, cqecl_ptr)
}

/// Hooks for DirectX 12.
pub struct ImguiDx12Hooks([MhHook; 3]);

impl ImguiDx12Hooks {
    /// Construct a set of [`MhHook`]s that will render UI via the
    /// provided [`ImguiRenderLoop`].
    ///
    /// The following functions are hooked:
    /// - `IDXGISwapChain3::Present`
    /// - `IDXGISwapChain3::ResizeBuffers`
    /// - `ID3D12CommandQueue::ExecuteCommandLists`
    ///
    /// # Safety
    ///
    /// yolo
    pub unsafe fn new<T>(t: T) -> Self
    where
        T: ImguiRenderLoop + Send + Sync + 'static,
    {
        let (
            dxgi_swap_chain_present_addr,
            dxgi_swap_chain_resize_buffers_addr,
            d3d12_command_queue_execute_command_lists_addr,
        ) = get_target_addrs();

        let hook_present = MhHook::new(
            dxgi_swap_chain_present_addr as *mut _,
            dxgi_swap_chain_present_impl as *mut _,
        )
        .expect("couldn't create IDXGISwapChain::Present hook");
        let hook_resize_buffers = MhHook::new(
            dxgi_swap_chain_resize_buffers_addr as *mut _,
            dxgi_swap_chain_resize_buffers_impl as *mut _,
        )
        .expect("couldn't create IDXGISwapChain::ResizeBuffers hook");
        let hook_cqecl = MhHook::new(
            d3d12_command_queue_execute_command_lists_addr as *mut _,
            d3d12_command_queue_execute_command_lists_impl as *mut _,
        )
        .expect("couldn't create ID3D12CommandQueue::ExecuteCommandLists hook");

        RENDER_LOOP.get_or_init(|| Box::new(t));

        TRAMPOLINES.get_or_init(|| {
            Trampolines {
                dxgi_swap_chain_present: mem::transmute::<*mut c_void, DXGISwapChainPresentType>(
                    hook_present.trampoline(),
                ),
                dxgi_swap_chain_resize_buffers: mem::transmute::<
                    *mut c_void,
                    DXGISwapChainResizeBuffersType,
                >(hook_resize_buffers.trampoline()),
                d3d12_command_queue_execute_command_lists: mem::transmute::<
                    *mut c_void,
                    D3D12CommandQueueExecuteCommandListsType,
                >(
                    hook_cqecl.trampoline()
                ),
            }
        });

        Self([hook_present, hook_resize_buffers, hook_cqecl])
    }
}

impl Hooks for ImguiDx12Hooks {
    fn from_render_loop<T>(t: T) -> Box<Self>
    where
        Self: Sized,
        T: ImguiRenderLoop + Send + Sync + 'static,
    {
        Box::new(unsafe { Self::new(t) })
    }

    fn hooks(&self) -> &[MhHook] {
        &self.0
    }

    unsafe fn unhook(&mut self) {
        TRAMPOLINES.take();
        PIPELINE.take().map(|p| p.into_inner().take());
        RENDER_LOOP.take(); // should already be null
        *INITIALIZATION_CONTEXT.lock() = InitializationContext::Empty;
    }
}
