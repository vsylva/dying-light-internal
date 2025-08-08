#![allow(dead_code, non_snake_case, non_camel_case_types, missing_docs)]

use std::{ffi::c_void, ptr::null_mut};

#[allow(non_camel_case_types)]
#[must_use]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MH_STATUS {
    MH_UNKNOWN = -1,

    MH_OK = 0,

    MH_ERROR_ALREADY_INITIALIZED,

    MH_ERROR_NOT_INITIALIZED,

    MH_ERROR_ALREADY_CREATED,

    MH_ERROR_NOT_CREATED,

    MH_ERROR_ENABLED,

    MH_ERROR_DISABLED,

    MH_ERROR_NOT_EXECUTABLE,

    MH_ERROR_UNSUPPORTED_FUNCTION,

    MH_ERROR_MEMORY_ALLOC,

    MH_ERROR_MEMORY_PROTECT,

    MH_ERROR_MODULE_NOT_FOUND,

    MH_ERROR_FUNCTION_NOT_FOUND,
}

unsafe extern "system" {
    pub fn MH_Initialize() -> MH_STATUS;
    pub fn MH_Uninitialize() -> MH_STATUS;
    pub fn MH_CreateHook(
        pTarget: *mut c_void,
        pDetour: *mut c_void,
        ppOriginal: *mut *mut c_void,
    ) -> MH_STATUS;
    pub fn MH_EnableHook(pTarget: *mut c_void) -> MH_STATUS;
    pub fn MH_QueueEnableHook(pTarget: *mut c_void) -> MH_STATUS;
    pub fn MH_DisableHook(pTarget: *mut c_void) -> MH_STATUS;
    pub fn MH_QueueDisableHook(pTarget: *mut c_void) -> MH_STATUS;
    pub fn MH_ApplyQueued() -> MH_STATUS;
}

impl MH_STATUS {
    pub fn ok_context(self) -> Result<(), MH_STATUS> {
        if self == MH_STATUS::MH_OK {
            Ok(())
        } else {
            Err(self)
        }
    }

    pub fn ok(self) -> Result<(), MH_STATUS> {
        if self == MH_STATUS::MH_OK {
            Ok(())
        } else {
            Err(self)
        }
    }
}

pub struct MhHook {
    addr: *mut c_void,
    hook_impl: *mut c_void,
    trampoline: *mut c_void,
}

impl MhHook {
    pub unsafe fn new(addr: *mut c_void, hook_impl: *mut c_void) -> Result<Self, MH_STATUS> {
        let mut trampoline = null_mut();
        MH_CreateHook(addr, hook_impl, &mut trampoline).ok_context()?;

        Ok(Self {
            addr,
            hook_impl,
            trampoline,
        })
    }

    pub fn trampoline(&self) -> *mut c_void {
        self.trampoline
    }

    pub unsafe fn queue_enable(&self) -> Result<(), MH_STATUS> {
        MH_QueueEnableHook(self.addr).ok_context()
    }

    pub unsafe fn queue_disable(&self) -> Result<(), MH_STATUS> {
        MH_QueueDisableHook(self.addr).ok_context()
    }
}
