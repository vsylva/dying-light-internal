//! General-purpose utilities. These are used across the [`crate`] but have
//! proven useful in client code as well.

use std::{
    mem::ManuallyDrop,
    sync::atomic::{AtomicU64, Ordering},
};

use windows::Win32::{
    Foundation::{HANDLE, HWND, RECT},
    Graphics::Direct3D12::{
        D3D12_FENCE_FLAG_NONE, D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
        D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
        D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATES,
        D3D12_RESOURCE_TRANSITION_BARRIER, ID3D12Device, ID3D12Fence, ID3D12Resource,
    },
    System::Threading::{CREATE_EVENT, CreateEventExW, WaitForSingleObjectEx},
    UI::WindowsAndMessaging::GetClientRect,
};

/// Helper for fallible [`windows`] APIs that have an out-param with a default
/// value.
///
/// # Example
///
/// ```
/// let swap_chain_desc = try_out_param(|sd| unsafe { self.swap_chain.GetDesc1(sd) })?;
/// ```
pub fn try_out_param<T, F, E, O>(mut f: F) -> Result<T, E>
where
    T: Default,
    F: FnMut(&mut T) -> Result<O, E>,
{
    let mut t: T = Default::default();
    match f(&mut t) {
        Ok(_) => Ok(t),
        Err(e) => Err(e),
    }
}

/// Helper for fallible [`windows`] APIs that have an optional pointer
/// out-param.
///
/// # Example
///
/// ```
/// let dev: ID3D12Device =
///     try_out_ptr(|v| unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, v) })
///         .expect("D3D12CreateDevice failed");
/// ```
pub fn try_out_ptr<T, F, E, O>(mut f: F) -> Result<T, E>
where
    F: FnMut(&mut Option<T>) -> Result<O, E>,
{
    let mut t: Option<T> = None;
    match f(&mut t) {
        Ok(_) => Ok(t.unwrap()),
        Err(e) => Err(e),
    }
}

/// Helper for fallible [`windows`] APIs that have an optional pointer
/// out-param and an optional pointer err-param.
///
/// # Example
///
/// ```
/// let blob: ID3DBlob = util::try_out_err_blob(|v, err_blob| {
///     D3D12SerializeRootSignature(
///         &root_signature_desc,
///         D3D_ROOT_SIGNATURE_VERSION_1_0,
///         v,
///         Some(err_blob),
///     )
/// })
/// .map_err(print_err_blob("Compiling vertex shader"))?;
/// ```
pub fn try_out_err_blob<T1, T2, F, E, O>(mut f: F) -> Result<T1, (E, T2)>
where
    F: FnMut(&mut Option<T1>, &mut Option<T2>) -> Result<O, E>,
{
    let mut t1: Option<T1> = None;
    let mut t2: Option<T2> = None;
    match f(&mut t1, &mut t2) {
        Ok(_) => Ok(t1.unwrap()),
        Err(e) => Err((e, t2.unwrap())),
    }
}

/// Helper for infallible APIs that have out-params, like OpenGL 3.
///
/// # Example
///
/// ```
/// let vertex_buffer = out_param(|x| unsafe { gl.GenBuffers(1, x) });
/// ```
pub fn out_param<T: Default, F>(f: F) -> T
where
    F: FnOnce(&mut T),
{
    let mut val = Default::default();
    f(&mut val);
    val
}

/// Helper that returns width and height of a given
/// [`windows::Win32::Foundation::HWND`].
pub fn win_size(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect).unwrap() };
    (rect.right - rect.left, rect.bottom - rect.top)
}

/// Creates a [`D3D12_RESOURCE_BARRIER`].
///
/// Use this function and the associated [`drop_barrier`] for correctly managing
/// barrier resources.
///
/// RAII was not used due to the complicated signature of
/// [`windows::Win32::Graphics::Direct3D12::ID3D12GraphicsCommandList::ResourceBarrier`].
pub fn create_barrier(
    resource: &ID3D12Resource,
    before: D3D12_RESOURCE_STATES,
    after: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: ManuallyDrop::new(Some(resource.clone())),
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: before,
                StateAfter: after,
            }),
        },
    }
}

/// Drops a [`D3D12_RESOURCE_BARRIER`].
///
/// Use this function and the associated [`create_barrier`] for correctly
/// managing barrier resources.
///
/// RAII was not used due to the complicated signature of
/// [`windows::Win32::Graphics::Direct3D12::ID3D12GraphicsCommandList::ResourceBarrier`].
pub fn drop_barrier(barrier: D3D12_RESOURCE_BARRIER) {
    let transition = ManuallyDrop::into_inner(unsafe { barrier.Anonymous.Transition });
    let _ = ManuallyDrop::into_inner(transition.pResource);
}

/// Wrapper around [`windows::Win32::Graphics::Direct3D12::ID3D12Fence`].
pub struct Fence {
    fence: ID3D12Fence,
    value: AtomicU64,
    event: HANDLE,
}

impl Fence {
    /// Construct the fence.
    pub fn new(device: &ID3D12Device) -> windows::core::Result<Self> {
        let fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }?;
        let value = AtomicU64::new(0);
        let event = unsafe { CreateEventExW(None, None, CREATE_EVENT(0), 0x1f0003) }?;

        Ok(Fence {
            fence,
            value,
            event,
        })
    }

    /// Retrieve the underlying fence object to pass to the D3D12 APIs.
    pub fn fence(&self) -> &ID3D12Fence {
        &self.fence
    }

    /// Retrieve the current fence value.
    pub fn value(&self) -> u64 {
        self.value.load(Ordering::SeqCst)
    }

    /// Atomically increase the fence value.
    pub fn incr(&self) {
        self.value.fetch_add(1, Ordering::SeqCst);
    }

    /// Wait for completion of the fence.
    pub fn wait(&self) -> windows::core::Result<()> {
        let value = self.value();
        unsafe {
            if self.fence.GetCompletedValue() < value {
                self.fence.SetEventOnCompletion(value, self.event)?;
                WaitForSingleObjectEx(self.event, u32::MAX, false);
            }
        }

        Ok(())
    }
}
