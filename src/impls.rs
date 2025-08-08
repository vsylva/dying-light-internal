// #![allow(unused)]
#![allow(static_mut_refs)]

use crate::{CameraFPPDI, ENGINE_DLL_INFO, GameDI, ModelObject, Vec2, Vec3};
use std::{
    mem::{MaybeUninit, transmute},
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
pub(crate) unsafe fn get_screen_width(game_di_p: *const GameDI) -> i32 {
    type Prototype = unsafe extern "system" fn(*const GameDI) -> i32;

    static mut PROC: usize = 0;
    static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?GetScreenWidth@IGame@@QEAAHXZ").unwrap();

        PROC_PTR.write(transmute(PROC));
    });

    PROC_PTR.assume_init()(game_di_p)
}

#[inline(always)]
pub(crate) unsafe fn get_screen_height(game_di_p: *const GameDI) -> i32 {
    type Prototype = unsafe extern "system" fn(*const GameDI) -> i32;

    static mut PROC: usize = 0;
    static mut FN: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(ENGINE_DLL_INFO.base, "?GetScreenHeight@IGame@@QEAAHXZ").unwrap();

        FN.write(transmute(PROC));
    });

    FN.assume_init()(game_di_p)
}

// #[inline(always)]
// pub(crate) unsafe fn get_world_position(
//     model_obj_p: *const ModelObject,
//     world_pos: *const Vec3<f32>,
// ) -> *const Vec3<f32> {
//     type Prototype =
//         unsafe extern "system" fn(*const ModelObject, *const Vec3<f32>) -> *const Vec3<f32>;

//     static mut PROC: usize = 0;
//     static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

//     static ONCE: Once = Once::new();

//     ONCE.call_once(|| {
//         PROC = get_proc_address(
//             ENGINE_DLL_INFO.base,
//             "?GetWorldPosition@IControlObject@@QEBA?AVvec3@@XZ",
//         )
//         .unwrap();

//         PROC_PTR.write(transmute(PROC));
//     });

//     PROC_PTR.assume_init()(model_obj_p, world_pos)
// }

#[inline(always)]
pub(crate) unsafe fn get_distance_to(
    model_obj_p: *const ModelObject,
    world_pos: *const Vec3<f32>,
) -> f32 {
    type Prototype = unsafe extern "system" fn(*const ModelObject, *const Vec3<f32>) -> f32;

    static mut PROC: usize = 0;
    static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetDistanceTo@IControlObject@@QEBAMAEBVvec3@@@Z",
        )
        .unwrap();

        PROC_PTR.write(transmute(PROC));
    });

    return PROC_PTR.assume_init()(model_obj_p, world_pos);
}

#[inline(always)]
pub(crate) unsafe fn raytest_to_target(
    model_obj_p: *const ModelObject,
    from: *const Vec3<f32>,
    to: *const Vec3<f32>,
    para: u8,
) -> i8 {
    type Prototype = unsafe extern "system" fn(
        *const ModelObject,
        *const ModelObject,
        *const Vec3<f32>,
        *const Vec3<f32>,
        u8,
    ) -> i8;

    static mut PROC: usize = 0;
    static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?RaytestToTarget@IControlObject@@QEAA_NPEBV1@AEBVvec3@@1E@Z",
        )
        .unwrap();

        PROC_PTR.write(transmute(PROC));
    });

    return PROC_PTR.assume_init()(model_obj_p, model_obj_p, from, to, para);
}

#[inline(always)]
pub(crate) unsafe fn get_bone_joint_pos(
    model_obj_p: *const ModelObject,
    world_pos: *const Vec3<f32>,
    index: u8,
) {
    type Prototype = unsafe extern "system" fn(*const ModelObject, *const Vec3<f32>, u8);

    static mut PROC: usize = 0;
    static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetBoneJointPos@IModelObject@@QEBA?AVvec3@@E@Z",
        )
        .unwrap();

        PROC_PTR.write(transmute(PROC));
    });

    PROC_PTR.assume_init()(model_obj_p, world_pos, index);
}

// #[inline(always)]
// pub(crate) unsafe fn point_to_screen_clamp_to_frustum(
//     camera_fpp_di_p: *const CameraFPPDI,
//     screen_pos: *const Vec2<f32>,
//     world_pos: *const Vec3<f32>,
// ) -> *const Vec2<f32> {
//     type Prototype = unsafe extern "system" fn(
//         *const CameraFPPDI,
//         *const Vec2<f32>,
//         *const Vec3<f32>,
//     ) -> *const Vec2<f32>;

//     static mut PROC: usize = 0;
//     static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

//     static ONCE: Once = Once::new();

//     ONCE.call_once(|| {
//         PROC = get_proc_address(
//             ENGINE_DLL_INFO.base,
//             "?PointToScreenClampToFrustum@IBaseCamera@@QEAA?BVvec3@@AEBV2@@Z",
//         )
//         .unwrap();

//         PROC_PTR.write(transmute(PROC));
//     });

//     PROC_PTR.assume_init()(camera_fpp_di_p, screen_pos, world_pos)
// }

#[inline(always)]
pub(crate) unsafe fn point_to_screen(
    camera_fpp_di_p: *const CameraFPPDI,
    screen_pos: *const Vec2<f32>,
    world_pos: *const Vec3<f32>,
) -> *const Vec2<f32> {
    type Prototype = unsafe extern "system" fn(
        *const CameraFPPDI,
        *const Vec2<f32>,
        *const Vec3<f32>,
    ) -> *const Vec2<f32>;

    static mut PROC: usize = 0;
    static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?PointToScreen@IBaseCamera@@QEAA?BVvec2@@AEBVvec3@@@Z",
        )
        .unwrap();

        PROC_PTR.write(transmute(PROC));
    });

    PROC_PTR.assume_init()(camera_fpp_di_p, screen_pos, world_pos)
}

// #[inline(always)]
// pub(crate) unsafe fn get_active_camera(
//     level_di_p: *const crate::LevelDI,
//     toggle: i64,
// ) -> *const CameraFPPDI {
//     type Prototype = unsafe extern "system" fn(*const crate::LevelDI, i64) -> *const CameraFPPDI;

//     static mut PROC: usize = 0;
//     static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

//     static ONCE: Once = Once::new();

//     ONCE.call_once(|| {
//         PROC = get_proc_address(
//             ENGINE_DLL_INFO.base,
//             "?GetActiveCamera@ILevel@@QEBAPEAVIBaseCamera@@XZ",
//         )
//         .unwrap();

//         PROC_PTR.write(transmute(PROC));
//     });

//     PROC_PTR.assume_init()(level_di_p, toggle)
// }

// #[inline(always)]
// pub(crate) unsafe fn get_active_level(game_di_p: *const GameDI) -> *const LevelDI {
//     type Prototype = unsafe extern "system" fn(*const GameDI) -> *const LevelDI;

//     static mut PROC: usize = 0;
//     static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

//     static ONCE: Once = Once::new();

//     ONCE.call_once(|| {
//         PROC = get_proc_address(
//             ENGINE_DLL_INFO.base,
//             "?GetActiveLevel@IGame@@QEAAPEAVILevel@@XZ",
//         )
//         .unwrap();

//         PROC_PTR.write(transmute(PROC));
//     });

//     PROC_PTR.assume_init()(game_di_p)
// }

#[inline(always)]
pub(crate) unsafe fn get_position(camera_fpp_di_p: *const CameraFPPDI) -> *const Vec3<f32> {
    type Prototype =
        unsafe extern "system" fn(*const CameraFPPDI, *const Vec3<f32>) -> *const Vec3<f32>;

    static mut PROC: usize = 0;
    static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetPosition@IBaseCamera@@QEBA?BVvec3@@XZ",
        )
        .unwrap();

        PROC_PTR.write(transmute(PROC));
    });

    let mut pos: Vec3<f32> = Vec3::default();

    return PROC_PTR.assume_init()(camera_fpp_di_p as *const CameraFPPDI, &mut pos);
}

#[inline(always)]
pub(crate) unsafe fn is_in_frustum(model_obj_p: *const ModelObject) -> i8 {
    type Prototype = unsafe extern "system" fn(*const ModelObject) -> i8;

    static mut PROC: usize = 0;
    static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?IsInFrustum@IControlObject@@QEBA_NXZ",
        )
        .unwrap();

        PROC_PTR.write(transmute(PROC));
    });

    return PROC_PTR.assume_init()(model_obj_p);
}

#[inline(always)]
pub(crate) unsafe fn get_objects_in_frustum(
    camera_fpp_di_p: *const CameraFPPDI,
    model_obj_p_array_p: *const crate::Array<*const ModelObject>,
    distance: f32,
) -> i8 {
    type Prototype = unsafe extern "system" fn(
        *const CameraFPPDI,
        *const crate::Array<*const ModelObject>,
        f32,
    ) -> i8;

    static mut PROC: usize = 0;
    static mut PROC_PTR: MaybeUninit<Prototype> = MaybeUninit::uninit();

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        PROC = get_proc_address(
            ENGINE_DLL_INFO.base,
            "?GetObjectsInFrustum@IBaseCamera@@QEAA_NPEAV?$vector@PEAVIControlObject@@@ttl@@M@Z",
        )
        .unwrap();

        PROC_PTR.write(transmute(PROC));
    });

    PROC_PTR.assume_init()(camera_fpp_di_p, model_obj_p_array_p, distance)
}
