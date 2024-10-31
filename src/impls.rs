#![allow(unused)]
use crate::{
    Array, CameraFPPDI, ENGINE_DLL_INFO, GameDI, LevelDI, ModelObject, Vec2Float, Vec3Float,
};
use std::{
    ptr::{addr_of, null},
    sync::Once,
};

#[inline(always)]
unsafe fn get_proc_address<S: AsRef<str>>(mod_handle: usize, proc_name: S) -> Option<usize> {
    let proc_addr = hudhook::windows::Win32::System::LibraryLoader::GetProcAddress(
        hudhook::windows::Win32::Foundation::HMODULE(mod_handle as isize),
        hudhook::windows::core::PCSTR::from_raw(format!("{}\0", proc_name.as_ref()).as_ptr()),
    )? as usize;

    if 0 == proc_addr {
        return None;
    }

    Some(proc_addr)
}

#[inline(always)]
pub unsafe fn acquire_input(game_di_p: *const GameDI, on: i8) -> i32 {
    type AcquireInput = unsafe extern "system" fn(*const GameDI, i8) -> i32;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const AcquireInput = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?AcquireInput@IGame@@QEAAX_N@Z").unwrap();

        PROC_PTR = addr_of!(PROC) as *const AcquireInput;
    });

    (*PROC_PTR)(game_di_p, on)
}

#[inline(always)]
pub unsafe fn get_screen_width(game_di_p: *const GameDI) -> i32 {
    type GetScreenWidth = unsafe extern "system" fn(*const GameDI) -> i32;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetScreenWidth = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?GetScreenWidth@IGame@@QEAAHXZ").unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetScreenWidth;
    });

    (*PROC_PTR)(game_di_p)
}

#[inline(always)]
pub unsafe fn get_screen_height(game_di_p: *const GameDI) -> i32 {
    type GetScreenHeight = unsafe extern "system" fn(*const GameDI) -> i32;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetScreenHeight = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?GetScreenHeight@IGame@@QEAAHXZ").unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetScreenHeight;
    });

    (*PROC_PTR)(game_di_p)
}

#[inline(always)]
pub unsafe fn exit_game(game_di_p: *const GameDI) {
    type ExitGame = unsafe extern "system" fn(*const GameDI);

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const ExitGame = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?ExitGame@IGame@@QEAAXXZ").unwrap();

        PROC_PTR = addr_of!(PROC) as *const ExitGame;
    });

    (*PROC_PTR)(game_di_p)
}

#[inline(always)]
pub unsafe fn get_active_level(game_di_p: *const GameDI) -> *const LevelDI {
    type GetActiveLevel = unsafe extern "system" fn(*const GameDI) -> *const LevelDI;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetActiveLevel = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetActiveLevel@IGame@@QEAAPEAVILevel@@XZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetActiveLevel;
    });

    (*PROC_PTR)(game_di_p)
}

#[inline(always)]
pub unsafe fn get_world_position(
    model_object_p: *const ModelObject,
    world_pos: *mut Vec3Float,
) -> *const Vec3Float {
    type GetWorldPosition =
        unsafe extern "system" fn(*const ModelObject, *mut Vec3Float) -> *const Vec3Float;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetWorldPosition = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetWorldPosition@IControlObject@@QEBA?AVvec3@@XZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetWorldPosition;
    });

    (*PROC_PTR)(model_object_p, world_pos)
}

#[inline(always)]
pub unsafe fn get_distance_to(model_object_p: *const ModelObject, pos: *const Vec3Float) -> f32 {
    type GetDistanceTo = unsafe extern "system" fn(*const ModelObject, *const Vec3Float) -> f32;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetDistanceTo = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetDistanceTo@IControlObject@@QEBAMAEBVvec3@@@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetDistanceTo;
    });

    return (*PROC_PTR)(model_object_p, pos);
}

#[inline(always)]
pub unsafe fn delete_all_mesh_part_cloths(model_object_p: *const ModelObject) -> i8 {
    type DeleteAllMeshPartCloths = unsafe extern "system" fn(*const ModelObject) -> i8;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const DeleteAllMeshPartCloths = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?DeleteAllMeshPartCloths@IModelObject@@QEAA_NXZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const DeleteAllMeshPartCloths;
    });

    return (*PROC_PTR)(model_object_p);
}

#[inline(always)]
pub unsafe fn raytest_to_target(
    model_object_p: *const ModelObject,
    to: *const Vec3Float,
    from: *const Vec3Float,
) -> i8 {
    type RaytestToTarget = unsafe extern "system" fn(
        *const ModelObject,
        *const ModelObject,
        *const Vec3Float,
        *const Vec3Float,
        u8,
    ) -> i8;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const RaytestToTarget = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?RaytestToTarget@IControlObject@@QEAA_NPEBV1@AEBVvec3@@1E@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const RaytestToTarget;
    });

    return (*PROC_PTR)(model_object_p, model_object_p, to, from, 4);
}

#[inline(always)]
pub unsafe fn get_bone_joint_pos(
    model_object_p: *const ModelObject,
    world_pos: *mut Vec3Float,
    index: u8,
) {
    type GetBoneJointPos = unsafe extern "system" fn(*const ModelObject, *mut Vec3Float, u8);

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetBoneJointPos = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetBoneJointPos@IModelObject@@QEBA?AVvec3@@E@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetBoneJointPos;
    });

    (*PROC_PTR)(model_object_p, world_pos, index);
}

#[inline(always)]
pub unsafe fn get_flags(model_object_p: *const ModelObject) -> i32 {
    type GetFlags = unsafe extern "system" fn(*const ModelObject) -> i32;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetFlags = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetFlags@IControlObject@@QEBA?AW4TYPE@COFlags@@XZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetFlags;
    });

    (*PROC_PTR)(model_object_p)
}

#[inline(always)]
pub unsafe fn point_to_screen_clamp_to_frustum(
    camera_fpp_di_p: *const CameraFPPDI,
    screen_pos: *mut Vec2Float,
    world_pos: *const Vec3Float,
) -> *const Vec2Float {
    type PointToScreenClampToFrustum = unsafe extern "system" fn(
        *const CameraFPPDI,
        *mut Vec2Float,
        *const Vec3Float,
    ) -> *const Vec2Float;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const PointToScreenClampToFrustum = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?PointToScreenClampToFrustum@IBaseCamera@@QEAA?BVvec3@@AEBV2@@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const PointToScreenClampToFrustum;
    });

    (*PROC_PTR)(camera_fpp_di_p, screen_pos, world_pos)
}

#[inline(always)]
pub unsafe fn point_to_screen(
    camera_fpp_di_p: *const CameraFPPDI,
    screen_pos: *mut Vec2Float,
    world_pos: *const Vec3Float,
) -> *const Vec2Float {
    type PointToScreen = unsafe extern "system" fn(
        *const CameraFPPDI,
        *mut Vec2Float,
        *const Vec3Float,
    ) -> *const Vec2Float;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const PointToScreen = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?PointToScreen@IBaseCamera@@QEAA?BVvec2@@AEBVvec3@@@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const PointToScreen;
    });

    (*PROC_PTR)(camera_fpp_di_p, screen_pos, world_pos)
}

#[inline(always)]
pub unsafe fn get_fov(camera_fpp_di_p: *const CameraFPPDI) -> f32 {
    type GetFov = unsafe extern "system" fn(*const CameraFPPDI) -> f32;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetFov = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?GetFOV@IBaseCamera@@QEAAMXZ").unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetFov;
    });

    return (*PROC_PTR)(camera_fpp_di_p as *const CameraFPPDI);
}

#[inline(always)]
pub unsafe fn get_position(camera_fpp_di_p: *const CameraFPPDI) -> *const Vec3Float {
    type GetPosition =
        unsafe extern "system" fn(*const CameraFPPDI, *const Vec3Float) -> *const Vec3Float;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetPosition = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetPosition@IBaseCamera@@QEBA?BVvec3@@XZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetPosition;
    });

    let mut pos: Vec3Float = Vec3Float::default();

    return (*PROC_PTR)(camera_fpp_di_p as *const CameraFPPDI, &mut pos);
}

#[inline(always)]
pub unsafe fn is_in_frustum(
    camera_fpp_di_p: *const CameraFPPDI,
    world_pos: *const Vec3Float,
) -> i8 {
    type IsInFrustum = unsafe extern "system" fn(*const CameraFPPDI, *const Vec3Float) -> i8;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const IsInFrustum = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?IsInFrustum@IBaseCamera@@QEAA_NAEBVvec3@@@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const IsInFrustum;
    });

    return (*PROC_PTR)(camera_fpp_di_p as *const CameraFPPDI, world_pos);
}

#[inline(always)]
pub unsafe fn get_objects_in_frustum(
    camera_fpp_di_p: *const CameraFPPDI,
    model_object_p_array_p: *const Array<*const ModelObject>,
    fov: f32,
) -> i8 {
    type GetObjectsInFrustum =
        unsafe extern "system" fn(*const CameraFPPDI, *const Array<*const ModelObject>, f32) -> i8;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetObjectsInFrustum = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetObjectsInFrustum@IBaseCamera@@QEAA_NPEAV?$vector@PEAVIControlObject@@@ttl@@M@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetObjectsInFrustum;
    });

    (*PROC_PTR)(camera_fpp_di_p, model_object_p_array_p, fov)
}

#[inline(always)]
pub unsafe fn set_fov(camera_fpp_di_p: *const CameraFPPDI, fov: f32) {
    type SetFov = unsafe extern "system" fn(*const CameraFPPDI, f32);

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const SetFov = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?SetFOV@IBaseCamera@@QEAAXM@Z").unwrap();

        PROC_PTR = addr_of!(PROC) as *const SetFov;
    });

    (*PROC_PTR)(camera_fpp_di_p, fov)
}

#[inline(always)]
pub unsafe fn remove_control_object(level_di: *const LevelDI, model_obj_p: *const ModelObject) {
    type RemoveControlObject = unsafe extern "system" fn(*const LevelDI, *const ModelObject);

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const RemoveControlObject = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?RemoveControlObject@ILevel@@QEAAXPEAVIControlObject@@@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const RemoveControlObject;
    });

    (*PROC_PTR)(level_di, model_obj_p)
}

#[inline(always)]
pub unsafe fn get_view_matrix(camera_fpp_di_p: *const CameraFPPDI) -> *const [[f32; 4]; 4] {
    type PointToScreenClampToFrustum =
        unsafe extern "system" fn(*const CameraFPPDI) -> *const [[f32; 4]; 4];

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const PointToScreenClampToFrustum = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetViewMatrix@IBaseCamera@@QEAAAEBVmtx34@@XZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const PointToScreenClampToFrustum;
    });

    (*PROC_PTR)(camera_fpp_di_p)
}

#[inline(always)]
pub unsafe fn get_projection_matrix(camera_fpp_di_p: *const CameraFPPDI) -> *const [[f32; 4]; 4] {
    type GetProjectionMatrix =
        unsafe extern "system" fn(*const CameraFPPDI) -> *const [[f32; 4]; 4];

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetProjectionMatrix = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetProjectionMatrix@IBaseCamera@@QEAAAEBVmtx44@@XZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetProjectionMatrix;
    });

    (*PROC_PTR)(camera_fpp_di_p)
}

#[inline(always)]
pub unsafe fn msg_send(game_di_p: *const GameDI, ptr: *const u8, len: u32) -> i32 {
    type MsgSend = unsafe extern "system" fn(*const GameDI, *const u8, u32) -> i32;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const MsgSend = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?MsgSend@IGame@@QEAAXIPEBEI@Z").unwrap();

        PROC_PTR = addr_of!(PROC) as *const MsgSend;
    });

    (*PROC_PTR)(game_di_p, ptr, len)
}

#[inline(always)]
pub unsafe fn set_safe_mode(game_di_p: *const GameDI, on: i8) {
    type SetSafeMode = unsafe extern "system" fn(*const GameDI, i8);

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const SetSafeMode = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?MsgSend@IGame@@QEAAXIPEBEI@Z").unwrap();

        PROC_PTR = addr_of!(PROC) as *const SetSafeMode;
    });

    (*PROC_PTR)(game_di_p, on)
}

#[inline(always)]
pub unsafe fn is_noise_receiver(model_object_p: *const ModelObject) -> i8 {
    type IsNoiseReceiver = unsafe extern "system" fn(*const ModelObject) -> i8;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const IsNoiseReceiver = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?IsNoiseReceiver@IControlObject@@UEAA_NXZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const IsNoiseReceiver;
    });

    return (*PROC_PTR)(model_object_p);
}

#[inline(always)]
pub unsafe fn get_tip_text(model_object_p: *const ModelObject) -> *const u8 {
    type GetTipText = unsafe extern "system" fn(*const ModelObject) -> *const u8;

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const GetTipText = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetTipText@IControlObject@@UEAA?AV?$string_base@D@ttl@@XZ",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const GetTipText;
    });

    return (*PROC_PTR)(model_object_p);
}

#[inline(always)]
pub unsafe fn set_world_position(model_object_p: *const ModelObject, world_pos: *const Vec3Float) {
    type SetWorldPosition = unsafe extern "system" fn(*const ModelObject, *const Vec3Float);

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const SetWorldPosition = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?SetWorldPosition@IControlObject@@QEAAXAEBVvec3@@@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const SetWorldPosition;
    });

    return (*PROC_PTR)(model_object_p, world_pos);
}

#[inline(always)]
pub unsafe fn add_control_object(level_di: *const LevelDI, model_obj_p: *const ModelObject) {
    type AddControlObject = unsafe extern "system" fn(*const LevelDI, *const ModelObject);

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const AddControlObject = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?AddControlObject@ILevel@@QEAAXPEAVIControlObject@@@Z",
        )
        .unwrap();

        PROC_PTR = addr_of!(PROC) as *const AddControlObject;
    });

    (*PROC_PTR)(level_di, model_obj_p)
}

#[inline(always)]
pub unsafe fn set_wind_power(level_di: *const LevelDI, power: f32) {
    type SetWindPower = unsafe extern "system" fn(*const LevelDI, f32);

    pub static mut PROC: usize = 0;
    pub static mut PROC_PTR: *const SetWindPower = null();

    pub static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?SetWindPower@ILevel@@QEAAXM@Z").unwrap();

        PROC_PTR = addr_of!(PROC) as *const SetWindPower;
    });

    (*PROC_PTR)(level_di, power)
}
