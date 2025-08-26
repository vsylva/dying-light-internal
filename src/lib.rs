#![allow(unsafe_op_in_unsafe_fn)]
#![allow(static_mut_refs)]

mod impls;

use hudhook::{
    imgui::{
        internal::RawCast,
        sys::{ImFontAtlas_AddFontFromFileTTF, ImFontAtlas_GetGlyphRangesChineseFull},
    },
    windows::Win32::{
        Foundation::HWND,
        Graphics::Gdi::ScreenToClient,
        System::Memory::IsBadReadPtr,
        UI::{
            Input::KeyboardAndMouse::GetAsyncKeyState,
            WindowsAndMessaging::{FindWindowA, GetCursorPos},
        },
    },
};

use impls::{
    get_bone_joint_pos, get_screen_height, get_screen_width, is_in_frustum, point_to_screen,
    raytest_to_target,
};
use std::{
    f32::consts::PI,
    ptr::{null, null_mut},
    thread::spawn,
};

use crate::impls::{get_distance_to, get_position};

static mut ENGINE_DLL_INFO: libmem::Module = libmem::Module {
    base: 0,
    end: 0,
    size: 0,
    path: String::new(),
    name: String::new(),
};

static mut GAME_DLL_INFO: libmem::Module = libmem::Module {
    base: 0,
    end: 0,
    size: 0,
    path: String::new(),
    name: String::new(),
};

static mut CGAME_PP: *const *const CGame = null_mut();

const BONE_LIST: [EBones; 15] = [
    EBones::Head,
    EBones::LClavicle,
    EBones::RClavicle,
    EBones::LUpperarm,
    EBones::RUpperarm,
    EBones::LForearm,
    EBones::RForearm,
    EBones::LHand,
    EBones::RHand,
    EBones::LThigh,
    EBones::RThigh,
    EBones::LCalf,
    EBones::RCalf,
    EBones::LFoot,
    EBones::RFoot,
];

const BONE_LISTS: &[&[EBones]] = &[
    &[
        EBones::Head,
        EBones::Neck,
        EBones::Spine3,
        EBones::Spine2,
        EBones::Spine1,
        EBones::Pelvis,
    ],
    &[
        EBones::Neck,
        EBones::LUpperarm,
        EBones::LForearm,
        EBones::LHand,
    ],
    &[
        EBones::Neck,
        EBones::RUpperarm,
        EBones::RForearm,
        EBones::RHand,
    ],
    &[EBones::Pelvis, EBones::LThigh, EBones::LCalf, EBones::LFoot],
    &[EBones::Pelvis, EBones::RThigh, EBones::RCalf, EBones::RFoot],
];

const NOP_8: [u8; 8] = [0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90];
const PITCH_ORIGINAL: [u8; 8] = [0xF3, 0x0F, 0x11, 0x83, 0x78, 0x11, 0x00, 0x00];
const YAW_ORIGINAL: [u8; 8] = [0xF3, 0x0F, 0x11, 0xB3, 0x74, 0x11, 0x00, 0x00];

trait Ptr {
    unsafe fn is_bad_read_ptr(&self, size: usize) -> bool;
}
impl<T> Ptr for *const T {
    unsafe fn is_bad_read_ptr(&self, size: usize) -> bool {
        IsBadReadPtr(Some(self.cast()), size).as_bool()
    }
}
impl<T> Ptr for *mut T {
    unsafe fn is_bad_read_ptr(&self, size: usize) -> bool {
        IsBadReadPtr(Some(self.cast()), size).as_bool()
    }
}

#[repr(C)]
#[derive(Default, PartialEq, Debug, Clone, Copy)]
enum ModelType {
    ZombieNormal,
    ZombieSpecial,
    ZombieHunter,
    SurvivorNormal,
    SurvivorSpecial,
    SurvivorShopkeeper,
    PlayerHuman,
    PlayerHunter,
    #[default]
    Other,
}
impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::ZombieNormal => write!(f, "丧尸"),
            ModelType::ZombieSpecial => write!(f, "特感"),
            ModelType::ZombieHunter => write!(f, "夜魔"),
            ModelType::SurvivorNormal => write!(f, "NPC"),
            ModelType::SurvivorShopkeeper => write!(f, "商贩"),
            ModelType::SurvivorSpecial => write!(f, "强盗"),
            ModelType::PlayerHuman => write!(f, "人类"),
            ModelType::PlayerHunter => write!(f, "猎手"),
            ModelType::Other => write!(f, "其他"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
#[repr(u8)]
enum EBones {
    Pelvis = 0,
    Spine = 1,
    Spine1 = 2,
    Spine2 = 3,
    Spine3 = 4,
    Neck = 5,
    Neck1 = 6,
    Neck2 = 7,
    #[default]
    Head = 8,
    EyeCamera = 9,
    LClavicle = 10,
    LUpperarm = 11,
    LForearm = 12,
    LHand = 13,
    RClavicle = 14,
    RUpperarm = 15,
    RForearm = 16,
    RHand = 17,
    LThigh = 18,
    RThigh = 19,
    LCalf = 20,
    RCalf = 21,
    LFoot = 22,
    RFoot = 23,
}
impl std::fmt::Display for EBones {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EBones::Pelvis => write!(f, "骨盆"),
            EBones::Spine => write!(f, "脊椎"),
            EBones::Spine1 => write!(f, "脊椎1"),
            EBones::Spine2 => write!(f, "脊椎2"),
            EBones::Spine3 => write!(f, "脊椎3"),
            EBones::Neck => write!(f, "脖子"),
            EBones::Neck1 => write!(f, "脖子1"),
            EBones::Neck2 => write!(f, "脖子2"),
            EBones::Head => write!(f, "头"),
            EBones::EyeCamera => write!(f, "眼相机"),
            EBones::LClavicle => write!(f, "左锁骨"),
            EBones::LUpperarm => write!(f, "左大臂"),
            EBones::LForearm => write!(f, "左小臂"),
            EBones::LHand => write!(f, "左手"),
            EBones::RClavicle => write!(f, "右锁骨"),
            EBones::RUpperarm => write!(f, "右大臂"),
            EBones::RForearm => write!(f, "右小臂"),
            EBones::RHand => write!(f, "右手"),
            EBones::LThigh => write!(f, "左大腿"),
            EBones::RThigh => write!(f, "右大腿"),
            EBones::LCalf => write!(f, "左小腿"),
            EBones::RCalf => write!(f, "右小腿"),
            EBones::LFoot => write!(f, "左脚"),
            EBones::RFoot => write!(f, "右脚"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
#[repr(i32)]
enum AimKeys {
    #[default]
    RMouseButton = 0x2,
    LCtrl = 0xA2,
    LShift = 0xA0,
    LAlt = 0xA4,
}

impl std::fmt::Display for AimKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AimKeys::LShift => write!(f, "左Shift"),
            AimKeys::LAlt => write!(f, "左Alt"),
            AimKeys::RMouseButton => write!(f, "鼠标右"),
            AimKeys::LCtrl => write!(f, "左Ctrl"),
        }
    }
}

#[derive(Debug, Default)]
struct World {
    game_p: *const CGame,
    game_di_p: *const GameDI,
    session_cooperative_di_p: *const SessionCooperativeDI,
    level_di_p: *const LevelDI,
    c_level_p: *const CLevel,
    local_client_di_p: *const LocalClientDI,
    player_di_p: *mut PlayerDI,
    player_c_model_obj_p: *const CModelObject,
    camera_manage_di_p: *const CameraManagerDI,
    camera_fpp_di_p: *const CameraFPPDI,
    player_world_pos_p: *mut Vec3<f32>,
    camera_angle_p: *mut Vec2<f32>,
}

static mut WORLD_MODEL_OBJ_ARRAY: Array<*const ModelObject> = Array {
    ptr: null(),
    len: 0,
    max: 0,
};

#[derive(Debug, Default, Clone)]
struct Obj {
    model_obj_p: *const ModelObject,
    c_model_obj_p: *const CModelObject,
    c_model_obj_logo_p: *const u32,
    c_model_obj_world_pos: Vec3<f32>,
    model_obj_health_p: *mut f32,
    model_obj_str_p: *const i8,
    model_obj_str: String,
    model_obj_type: ModelType,
}

#[derive(Debug, Default)]
#[repr(C)]
struct Array<T> {
    ptr: *const T,
    len: u32,
    max: u32,
}

unsafe impl Send for Array<*const ModelObject> {}
unsafe impl Sync for Array<*const ModelObject> {}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Vec2<T> {
    x: T,
    y: T,
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
struct Vec3<T> {
    x: T,
    y: T,
    z: T,
}

#[repr(C)]
struct CGame;

#[repr(C)]
struct GameDI;

#[repr(C)]
struct ModelObject;

#[repr(C)]
struct CModelObject;

#[repr(C)]
struct SessionCooperativeDI;

#[repr(C)]
struct LevelDI;

#[repr(C)]
struct CLevel;

#[repr(C)]
struct LocalClientDI;

#[repr(C)]
struct PlayerDI;

#[repr(C)]
struct CameraManagerDI;

#[repr(C)]
struct CameraFPPDI;

#[repr(C)]
struct HealthModule;

#[derive(Debug, Clone)]
struct Game {
    game_window: HWND,

    is_menu_on: bool,

    toggle_filter_zombie_normal: bool,
    toggle_filter_zombie_special: bool,
    toggle_filter_zombie_hunter: bool,
    toggle_filter_survivor_normal: bool,
    toggle_filter_survivor_special: bool,
    toggle_filter_survivor_shopkeeper: bool,
    toggle_filter_player_human: bool,
    toggle_filter_player_hunter: bool,
    toggle_filter_other: bool,

    toggle_draw_model_type_name: bool,
    toggle_draw_bones: bool,
    toggle_draw_visible_line: bool,
    toggle_draw_type_data: bool,
    toggle_draw_logo: bool,
    toggle_draw_model_obj_p: bool,
    toggle_draw_model_obj_p_array: bool,
    toggle_draw_world_data: bool,

    aim_vk_code: i32,
    aim_is_key_down: bool,
    aim_is_mouse_patched: bool,
    aim_locking_model_obj_p: *const ModelObject,
    aim_best_closest_distance: f32,
    aim_best_closest_model_obj_p: *const ModelObject,
    aim_mouse_yaw_p: usize,
    aim_mouse_pitch_p: usize,
    aim_fov: f32,
    aim_toggle_draw_fov: bool,
    aim_toggle: bool,
    aim_selected_key: AimKeys,
    aim_key_list: [AimKeys; 4],
    aim_selected_bone: EBones,

    aim_toggle_filter_zombie_normal: bool,
    aim_toggle_filter_zombie_special: bool,
    aim_toggle_filter_zombie_hunter: bool,
    aim_toggle_filter_survivor_special: bool,
    aim_toggle_filter_player_human: bool,
    aim_toggle_filter_player_hunter: bool,

    color_zombie_normal: [f32; 4],
    color_zombie_special: [f32; 4],
    color_zombie_hunter: [f32; 4],
    color_survivor_nomal: [f32; 4],
    color_survivor_special: [f32; 4],
    color_survivor_shopkeeper: [f32; 4],
    color_player_human: [f32; 4],
    color_player_hunter: [f32; 4],
    color_other: [f32; 4],
}

impl Default for Game {
    fn default() -> Self {
        Self {
            aim_vk_code: AimKeys::RMouseButton as i32,

            aim_is_key_down: false,

            aim_is_mouse_patched: false,

            aim_locking_model_obj_p: null(),

            aim_best_closest_distance: f32::MAX,
            aim_best_closest_model_obj_p: null(),

            aim_mouse_yaw_p: 0,
            aim_mouse_pitch_p: 0,

            aim_toggle_filter_zombie_normal: false,
            aim_toggle_filter_zombie_special: false,
            aim_toggle_filter_survivor_special: false,
            aim_toggle_filter_zombie_hunter: false,
            aim_toggle_filter_player_human: false,
            aim_toggle_filter_player_hunter: false,

            game_window: HWND(0),
            is_menu_on: false,

            aim_selected_bone: EBones::Head,

            toggle_filter_zombie_normal: false,
            toggle_filter_zombie_special: false,
            toggle_filter_zombie_hunter: false,
            toggle_filter_survivor_normal: false,
            toggle_filter_survivor_special: false,
            toggle_filter_survivor_shopkeeper: false,
            toggle_filter_player_human: false,
            toggle_filter_player_hunter: false,
            toggle_filter_other: false,

            toggle_draw_bones: false,
            toggle_draw_visible_line: false,
            toggle_draw_model_type_name: false,
            toggle_draw_type_data: false,
            toggle_draw_logo: false,
            toggle_draw_model_obj_p: false,
            toggle_draw_model_obj_p_array: false,
            toggle_draw_world_data: false,

            aim_toggle: false,
            aim_toggle_draw_fov: false,
            aim_fov: 150.0,
            aim_key_list: [
                AimKeys::RMouseButton,
                AimKeys::LCtrl,
                AimKeys::LShift,
                AimKeys::LAlt,
            ],
            aim_selected_key: AimKeys::RMouseButton,

            color_zombie_normal: [1.0, 1.0, 1.0, 1.0], // 白色
            color_zombie_special: [0.7569, 1.0, 0.7569, 1.0], // 深海绿
            color_zombie_hunter: [1.0, 0.0, 1.0, 1.0], // 紫红色

            color_survivor_nomal: [0.2549, 0.4118, 0.8824, 1.0], // 皇家蓝
            color_survivor_special: [1.0, 1.0, 0.0, 1.0],        // 黄色
            color_survivor_shopkeeper: [0.0, 1.0, 1.0, 1.0],     // 青色

            color_player_human: [0.0, 1.0, 0.0, 1.0], // 绿色
            color_player_hunter: [1.0, 0.0, 0.0, 1.0], // 红色
            color_other: [1.0, 0.0, 0.0, 1.0],        // 白色
        }
    }
}

unsafe impl Send for Game {}

unsafe impl Sync for Game {}

impl hudhook::ImguiRenderLoop for Game {
    unsafe fn initialize<'a>(
        &'a mut self,
        ctx: &mut hudhook::imgui::Context,
        _: &'a mut dyn hudhook::RenderContext,
    ) {
        self.game_window = FindWindowA(hudhook::windows::core::s!("techland_game_class"), None);

        ImFontAtlas_AddFontFromFileTTF(
            ctx.fonts().raw_mut(),
            "C:\\windows\\fonts\\simhei.ttf\0".as_ptr().cast(),
            25.0,
            std::ptr::null(),
            ImFontAtlas_GetGlyphRangesChineseFull(ctx.fonts().raw_mut()),
        );

        ctx.style_mut().use_light_colors();
        ctx.set_ini_filename(None);
    }

    unsafe fn render(&mut self, ctx: &mut hudhook::imgui::Context) {
        static mut IS_KEY_OPEN_MENU_DOWN: bool = false;
        if GetAsyncKeyState(0xC0) & 0x8000u16 as i16 != 0 {
            if !IS_KEY_OPEN_MENU_DOWN {
                IS_KEY_OPEN_MENU_DOWN = true;

                self.is_menu_on = !self.is_menu_on;

                ctx.io_mut().mouse_draw_cursor = self.is_menu_on;
            }
        } else if IS_KEY_OPEN_MENU_DOWN {
            IS_KEY_OPEN_MENU_DOWN = false;
        }

        let mut mouse_pos: hudhook::windows::Win32::Foundation::POINT =
            hudhook::windows::Win32::Foundation::POINT { x: 0, y: 0 };

        GetCursorPos(&mut mouse_pos).unwrap_or_default();
        ScreenToClient(self.game_window, &mut mouse_pos);

        ctx.io_mut().mouse_pos[0] = mouse_pos.x as f32;
        ctx.io_mut().mouse_pos[1] = mouse_pos.y as f32;

        static mut IS_MOUSE_LEFT_DOWN: bool = false;
        if GetAsyncKeyState(0x1) & 0x8000u16 as i16 != 0 {
            IS_MOUSE_LEFT_DOWN = true;

            ctx.io_mut().mouse_down[0] = true;
        } else if IS_MOUSE_LEFT_DOWN {
            IS_MOUSE_LEFT_DOWN = false;

            ctx.io_mut().mouse_down[0] = false;
        }

        let ui = ctx.frame();
        on_frame_draw(self, ui);

        if !self.is_menu_on {
            return;
        }

        ui.window("[~]键\t1.52.0.0")
            .title_bar(true)
            .size([600.0, 450.0], hudhook::imgui::Condition::FirstUseEver)
            .build(|| {
                if let Some(val) = ui.tab_bar("##bar") {
                    on_frame_draw_ui(self, ui);

                    val.end();
                }
            });
    }
}

// ModelObj
// 0x104 0x114 0x124 world_pos
// 0x78  ObjectType,  16 HumanAI  17 PlayerDI
// 0x7C  ObjectType2, 17 HumanAI  17 PlayerDI
// 0x90 是-18以后的0x78
unsafe fn on_frame_draw(game: &mut Game, ui: &hudhook::imgui::Ui) {
    let world = match get_world() {
        Some(val) => val,
        None => return,
    };

    if game.toggle_draw_model_obj_p_array {
        ui.get_background_draw_list().add_text(
            [0.0, 0.0],
            game.color_zombie_normal,
            format!(
                "ptr:{:p}\nlen:{}\nmax:{}",
                WORLD_MODEL_OBJ_ARRAY.ptr, WORLD_MODEL_OBJ_ARRAY.len, WORLD_MODEL_OBJ_ARRAY.max
            ),
        );
    }

    if game.toggle_draw_world_data {
        ui.get_background_draw_list().add_text(
            [0.0, 0.0],
            game.color_zombie_normal,
            format!("{:#?}", world),
        );
    }

    if game.aim_toggle_draw_fov {
        ui.get_background_draw_list()
            .add_circle(
                [
                    get_screen_width(world.game_di_p) as f32 / 2.0,
                    get_screen_height(world.game_di_p) as f32 / 2.0,
                ],
                game.aim_fov,
                game.color_zombie_normal,
            )
            .build();
    }

    for index in 0..WORLD_MODEL_OBJ_ARRAY.len {
        let model_obj_pp = WORLD_MODEL_OBJ_ARRAY.ptr.add(index as usize);
        if model_obj_pp.is_bad_read_ptr(8) {
            continue;
        }

        let mut model_obj_p = model_obj_pp.read();
        if model_obj_p.is_bad_read_ptr(8) {
            continue;
        }

        model_obj_p = model_obj_p.byte_sub(0x18);
        if model_obj_p.is_bad_read_ptr(8) {
            continue;
        }

        let obj = match get_obj(model_obj_p) {
            Some(val) => val,
            None => continue,
        };

        if obj.c_model_obj_p == world.player_c_model_obj_p {
            continue;
        }

        if is_in_frustum(obj.model_obj_p) == 0 {
            continue;
        }

        let (filter, color) = match obj.model_obj_type {
            ModelType::ZombieNormal => (game.toggle_filter_zombie_normal, game.color_zombie_normal),
            ModelType::ZombieSpecial => {
                (game.toggle_filter_zombie_special, game.color_zombie_special)
            }
            ModelType::ZombieHunter => (game.toggle_filter_zombie_hunter, game.color_zombie_hunter),
            ModelType::SurvivorNormal => (
                game.toggle_filter_survivor_normal,
                game.color_survivor_nomal,
            ),
            ModelType::SurvivorSpecial => (
                game.toggle_filter_survivor_special,
                game.color_survivor_special,
            ),
            ModelType::SurvivorShopkeeper => (
                game.toggle_filter_survivor_shopkeeper,
                game.color_survivor_shopkeeper,
            ),
            ModelType::PlayerHuman => (game.toggle_filter_player_human, game.color_player_human),
            ModelType::PlayerHunter => (game.toggle_filter_player_hunter, game.color_player_hunter),
            ModelType::Other => (game.toggle_filter_other, game.color_other),
        };

        if !filter {
            continue;
        }

        let mut screen_pos: Vec2<f32> = Vec2::default();
        point_to_screen(
            world.camera_fpp_di_p,
            &mut screen_pos,
            &obj.c_model_obj_world_pos,
        );

        if game.aim_toggle {
            aim_update_obj(game, &world, &obj);
        }

        //  ui.get_background_draw_list() 不能 let，否则在下次调用 ui.get_background_draw_list()时会闪退
        if game.toggle_draw_model_type_name {
            let data = format!(
                "{}  {:.2}",
                obj.model_obj_type,
                get_distance_to(obj.model_obj_p, world.player_world_pos_p),
            );

            ui.get_background_draw_list()
                .add_text([screen_pos.x, screen_pos.y], color, data);
        }

        if game.toggle_draw_bones {
            draw_bones(ui, &world, &obj, color);
        }

        if game.toggle_draw_visible_line {
            let bone_world_pos: Vec3<f32> = Vec3::default();
            get_bone_joint_pos(
                obj.model_obj_p,
                &bone_world_pos,
                game.aim_selected_bone as u8,
            );

            if raytest_to_target(
                obj.model_obj_p,
                obj.model_obj_p,
                get_position(world.camera_fpp_di_p),
                &bone_world_pos,
                4,
            ) != 0
            {
                let mut bone_screen_pos: Vec2<f32> = Vec2::default();
                point_to_screen(world.camera_fpp_di_p, &mut bone_screen_pos, &bone_world_pos);

                ui.get_background_draw_list()
                    .add_line(
                        [
                            get_screen_width(world.game_di_p) as f32 / 2.0,
                            get_screen_height(world.game_di_p) as f32,
                        ],
                        [bone_screen_pos.x, bone_screen_pos.y],
                        color,
                    )
                    .thickness(2.0)
                    .build();
            }
        }

        if game.toggle_draw_type_data {
            ui.get_background_draw_list().add_text(
                [screen_pos.x, screen_pos.y],
                color,
                obj.model_obj_str.as_str(),
            );
        }

        if game.toggle_draw_logo {
            ui.get_background_draw_list().add_text(
                [screen_pos.x, screen_pos.y],
                color,
                format!("{:#X?}", obj.c_model_obj_logo_p.read()),
            );
        }

        if game.toggle_draw_model_obj_p {
            ui.get_background_draw_list().add_text(
                [screen_pos.x, screen_pos.y],
                color,
                format!(
                    "model_obj_p: {:p}\nc_model_obj_p: {:p}",
                    obj.model_obj_p, obj.c_model_obj_p,
                ),
            );
        }
    }

    if game.aim_toggle {
        aim_lock_obj(game, &world, game.aim_selected_bone as u8);
    }
}

unsafe fn on_frame_draw_ui(game: &mut Game, ui: &hudhook::imgui::Ui) {
    if let Some(val) = ui.tab_item("过滤") {
        ui.slider(
            "字体缩放##FontGlobalScale",
            0.5,
            1.5,
            &mut (*hudhook::imgui::sys::igGetIO()).FontGlobalScale,
        );

        // ZombieNormal
        ui.checkbox(
            "丧尸##toggle_filter_zombie_normal",
            &mut game.toggle_filter_zombie_normal,
        );
        ui.same_line();
        ui.color_edit4_config("##color_zombie_normal", &mut game.color_zombie_normal)
            .inputs(false)
            .build();

        // ZombieSpecial
        ui.checkbox(
            "特感##toggle_filter_zombie_special",
            &mut game.toggle_filter_zombie_special,
        );
        ui.same_line();
        ui.color_edit4_config("##color_zombie_special", &mut game.color_zombie_special)
            .inputs(false)
            .build();

        // ZombieHunter
        ui.checkbox(
            "夜魔##toggle_filter_zombie_hunter",
            &mut game.toggle_filter_zombie_hunter,
        );
        ui.same_line();
        ui.color_edit4_config("##color_zombie_hunter", &mut game.color_zombie_hunter)
            .inputs(false)
            .build();

        // SurvivorNormal
        ui.checkbox(
            "NPC##toggle_filter_survivor_normal",
            &mut game.toggle_filter_survivor_normal,
        );
        ui.same_line();
        ui.color_edit4_config("##color_survivor_nomal", &mut game.color_survivor_nomal)
            .inputs(false)
            .build();

        // SurvivorSpecial
        ui.checkbox(
            "强盗##toggle_filter_survivor_special,",
            &mut game.toggle_filter_survivor_special,
        );
        ui.same_line();
        ui.color_edit4_config("##color_survivor_special", &mut game.color_survivor_special)
            .inputs(false)
            .build();

        // SurvivorShopkeeper
        ui.checkbox(
            "商贩##toggle_filter_survivor_shopkeeper",
            &mut game.toggle_filter_survivor_shopkeeper,
        );
        ui.same_line();
        ui.color_edit4_config(
            "##color_survivor_shopkeeper",
            &mut game.color_survivor_shopkeeper,
        )
        .inputs(false)
        .build();

        // PlayerHuman
        ui.checkbox(
            "人类##toggle_filter_player_human",
            &mut game.toggle_filter_player_human,
        );
        ui.same_line();
        ui.color_edit4_config("##color_player_human", &mut game.color_player_human)
            .inputs(false)
            .build();

        // PlayerHunter
        ui.checkbox(
            "猎手##toggle_filter_player_hunter",
            &mut game.toggle_filter_player_hunter,
        );
        ui.same_line();
        ui.color_edit4_config("##color_player_hunter", &mut game.color_player_hunter)
            .inputs(false)
            .build();

        // Other
        ui.checkbox("其他##witch_filter_other", &mut game.toggle_filter_other);
        ui.same_line();
        ui.color_edit4_config("##color_other", &mut game.color_other)
            .inputs(false)
            .build();

        val.end();
    }

    if let Some(val) = ui.tab_item("绘制") {
        ui.checkbox(
            "类型##toggle_draw_model_type_name",
            &mut game.toggle_draw_model_type_name,
        );

        ui.checkbox("骨骼##toggle_draw_bones", &mut game.toggle_draw_bones);

        ui.checkbox(
            "可视线##toggle_draw_visible_line",
            &mut game.toggle_draw_visible_line,
        );

        ui.checkbox(
            "模型名##toggle_draw_type_data",
            &mut game.toggle_draw_type_data,
        );

        ui.checkbox("特征标志##toggle_draw_logo", &mut game.toggle_draw_logo);

        ui.checkbox(
            "对象地址##toggle_draw_model_obj_p",
            &mut game.toggle_draw_model_obj_p,
        );

        ui.checkbox(
            "对象地址数组##toggle_draw_model_obj_p_array",
            &mut game.toggle_draw_model_obj_p_array,
        );

        ui.checkbox(
            "世界地址##toggle_draw_world_data",
            &mut game.toggle_draw_world_data,
        );

        val.end();
    }

    if let Some(val) = ui.tab_item("自瞄") {
        ui.checkbox("开启##toggle_aim", &mut game.aim_toggle);

        if let Some(cb) =
            ui.begin_combo("按键##aim_selected_key", game.aim_selected_key.to_string())
        {
            for current in game.aim_key_list.as_slice() {
                if game.aim_selected_key == *current {
                    ui.set_item_default_focus();
                }

                if ui
                    .selectable_config(current.to_string())
                    .selected(game.aim_selected_key == *current)
                    .build()
                {
                    game.aim_selected_key = *current;
                    game.aim_vk_code = game.aim_selected_key as i32;
                }
            }
            cb.end();
        }

        if let Some(cb) = ui.begin_combo(
            "部位##aim_selected_bone",
            game.aim_selected_bone.to_string(),
        ) {
            for current in BONE_LIST.as_slice() {
                if game.aim_selected_bone == *current {
                    ui.set_item_default_focus();
                }

                if ui
                    .selectable_config(current.to_string())
                    .selected(game.aim_selected_bone == *current)
                    .build()
                {
                    game.aim_selected_bone = *current;
                }
            }
            cb.end();
        }

        ui.checkbox("FOV##toggle_aim_fov", &mut game.aim_toggle_draw_fov);
        ui.same_line();
        ui.slider("##aim_fov", 50.0, 500.0, &mut game.aim_fov);

        ui.checkbox(
            "丧尸##toggle_aim_filter_zombie_normal",
            &mut game.aim_toggle_filter_zombie_normal,
        );

        ui.checkbox(
            "特感##toggle_aim_filter_zombie_special",
            &mut game.aim_toggle_filter_zombie_special,
        );

        ui.checkbox(
            "夜魔##aim_toggle_filter_zombie_hunter",
            &mut game.aim_toggle_filter_zombie_hunter,
        );

        ui.checkbox(
            "强盗##toggle_aim_filter_survivor_special",
            &mut game.aim_toggle_filter_survivor_special,
        );

        ui.checkbox(
            "人类##toggle_aim_filter_player_human",
            &mut game.aim_toggle_filter_player_human,
        );

        ui.checkbox(
            "猎手##toggle_aim_filter_player_hunter",
            &mut game.aim_toggle_filter_player_hunter,
        );

        val.end();
    }
}

#[inline(always)]
unsafe fn draw_bones(ui: &hudhook::imgui::Ui, world: &World, obj: &Obj, color: [f32; 4]) {
    let mut previous_world_pos: Vec3<f32> = Vec3::default();

    let mut current_world_pos: Vec3<f32> = Vec3::default();

    for bone_list in BONE_LISTS {
        previous_world_pos.x = 0.0;
        previous_world_pos.y = 0.0;
        previous_world_pos.z = 0.0;

        for bone in *bone_list {
            get_bone_joint_pos(obj.model_obj_p, &mut current_world_pos, *bone as u8);

            if previous_world_pos.x == 0.0
                && previous_world_pos.y == 0.0
                && previous_world_pos.z == 0.0
            {
                previous_world_pos = current_world_pos;
                continue;
            }

            let mut previous_screen_pos: Vec2<f32> = Vec2::default();

            let mut current_screen_pos: Vec2<f32> = Vec2::default();

            point_to_screen(
                world.camera_fpp_di_p,
                &mut previous_screen_pos,
                &mut previous_world_pos,
            );

            point_to_screen(
                world.camera_fpp_di_p,
                &mut current_screen_pos,
                &mut current_world_pos,
            );

            ui.get_background_draw_list()
                .add_line(
                    [previous_screen_pos.x, previous_screen_pos.y],
                    [current_screen_pos.x, current_screen_pos.y],
                    color,
                )
                .thickness(1.5)
                .build();

            previous_world_pos = current_world_pos;
        }
    }
}

#[inline(always)]
unsafe fn aim_update_obj(game: &mut Game, world: &World, obj: &Obj) {
    if !match obj.model_obj_type {
        ModelType::Other | ModelType::SurvivorNormal | ModelType::SurvivorShopkeeper => return,
        ModelType::ZombieNormal => game.aim_toggle_filter_zombie_normal,
        ModelType::ZombieSpecial => game.aim_toggle_filter_zombie_special,
        ModelType::ZombieHunter => game.aim_toggle_filter_zombie_hunter,
        ModelType::SurvivorSpecial => game.aim_toggle_filter_survivor_special,
        ModelType::PlayerHuman => game.aim_toggle_filter_player_human,
        ModelType::PlayerHunter => game.aim_toggle_filter_player_hunter,
    } {
        return;
    }

    let mut screen_pos: Vec2<f32> = Vec2::default();

    let world_pos: Vec3<f32> = Vec3::default();

    get_bone_joint_pos(obj.model_obj_p, &world_pos, game.aim_selected_bone as u8);

    point_to_screen(world.camera_fpp_di_p, &mut screen_pos, &world_pos);

    let pow_x = (screen_pos.x - get_screen_width(world.game_di_p) as f32 / 2.0).powf(2.0);

    let pow_y = (screen_pos.y - get_screen_height(world.game_di_p) as f32 / 2.0).powf(2.0);

    let distance = (pow_x + pow_y).sqrt();

    if distance > game.aim_fov || distance > game.aim_best_closest_distance {
        return;
    }

    game.aim_best_closest_distance = distance;
    game.aim_best_closest_model_obj_p = obj.model_obj_p;
}

#[inline(always)]
unsafe fn aim_lock_obj(game: &mut Game, world: &World, selected_bone: u8) {
    if !game.aim_is_key_down {
        if game.aim_is_mouse_patched {
            game.aim_is_mouse_patched = false;
            mouse_unpatch(game);
        }

        if game.aim_best_closest_distance == f32::MAX {
            return;
        }
    }

    if GetAsyncKeyState(game.aim_vk_code) & 0x8000u16 as i16 != 0 {
        if !game.aim_is_key_down {
            game.aim_is_key_down = true;

            game.aim_locking_model_obj_p = game.aim_best_closest_model_obj_p;

            if !game.aim_is_mouse_patched {
                game.aim_is_mouse_patched = true;
                mouse_patch(game);
            }
        }

        if game.aim_locking_model_obj_p.is_bad_read_ptr(8) {
            game.aim_is_key_down = false;

            if game.aim_is_mouse_patched {
                game.aim_is_mouse_patched = false;
                mouse_unpatch(game);
            }

            return;
        }

        if game.aim_locking_model_obj_p.is_bad_read_ptr(8) {
            game.aim_is_key_down = false;

            if game.aim_is_mouse_patched {
                game.aim_is_mouse_patched = false;
                mouse_unpatch(game);
            }

            return;
        }

        if let None = get_obj(game.aim_locking_model_obj_p) {
            game.aim_is_key_down = false;

            if game.aim_is_mouse_patched {
                game.aim_is_mouse_patched = false;
                mouse_unpatch(game);
            }

            return;
        }

        let mut world_pos: Vec3<f32> = Vec3::default();

        get_bone_joint_pos(game.aim_locking_model_obj_p, &mut world_pos, selected_bone);

        let mut pos: Vec3<f32> = Vec3::default();

        let camera_world_pos_p = get_position(world.camera_fpp_di_p);

        pos.x = world_pos.x - camera_world_pos_p.read().x;
        pos.y = world_pos.z - camera_world_pos_p.read().z;
        pos.z = world_pos.y - camera_world_pos_p.read().y;

        let mut yaw = (pos.y / pos.x).atan() * 180.0 / PI;
        let pitch = (pos.z / (pos.x.powi(2) + pos.y.powi(2)).sqrt()).atan() * 180.0 / PI;

        if yaw < 0.0 && pos.y > 0.0 {
            yaw += 180.0;
        }

        if yaw > 0.0 && pos.y < 0.0 {
            yaw -= 180.0;
        }

        (*world.camera_angle_p).x = yaw;
        (*world.camera_angle_p).y = pitch;
    } else {
        game.aim_is_key_down = false;

        if game.aim_is_mouse_patched {
            game.aim_is_mouse_patched = false;
            mouse_unpatch(game);
        }
    }

    game.aim_best_closest_distance = f32::MAX;
}

#[inline(always)]
unsafe fn mouse_patch(game: &mut Game) {
    libmem::memory::write_memory_ex(
        &libmem::process::get_process().unwrap(),
        game.aim_mouse_yaw_p,
        NOP_8.as_slice(),
    );

    libmem::memory::write_memory_ex(
        &libmem::process::get_process().unwrap(),
        game.aim_mouse_pitch_p,
        NOP_8.as_slice(),
    );
}

#[inline(always)]
unsafe fn mouse_unpatch(game: &mut Game) {
    libmem::memory::write_memory_ex(
        &libmem::process::get_process().unwrap(),
        game.aim_mouse_yaw_p,
        YAW_ORIGINAL.as_slice(),
    );

    libmem::memory::write_memory_ex(
        &libmem::process::get_process().unwrap(),
        game.aim_mouse_pitch_p,
        PITCH_ORIGINAL.as_slice(),
    );
}

#[inline(always)]
unsafe fn get_world() -> Option<World> {
    let mut world = World {
        game_p: null(),
        game_di_p: null(),
        session_cooperative_di_p: null(),
        level_di_p: null(),
        c_level_p: null(),
        local_client_di_p: null(),
        player_di_p: null_mut(),
        player_c_model_obj_p: null(),

        camera_manage_di_p: null(),
        camera_fpp_di_p: null(),

        player_world_pos_p: null_mut(),
        camera_angle_p: null_mut(),
    };

    // CGame
    world.game_p = CGAME_PP.read();

    // GameDI
    let game_di_pp = world.game_p.byte_add(0x98).cast::<*const GameDI>();
    if game_di_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.game_di_p = game_di_pp.read();
    if world.game_di_p.is_bad_read_ptr(8) {
        return None;
    }

    // SessionCooperativeDI
    let session_cooperative_di_pp = world
        .game_di_p
        .byte_add(0x540)
        .cast::<*const SessionCooperativeDI>();
    if session_cooperative_di_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.session_cooperative_di_p = session_cooperative_di_pp.read();
    if world.session_cooperative_di_p.is_bad_read_ptr(8) {
        return None;
    }

    // LevelDI
    let level_di_pp = world
        .session_cooperative_di_p
        .byte_add(0xB0)
        .cast::<*const LevelDI>();
    if level_di_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.level_di_p = level_di_pp.read();
    if world.level_di_p.is_bad_read_ptr(8) {
        return None;
    }

    // CLevel
    let c_level_pp = world.level_di_p.byte_add(0x8).cast::<*const CLevel>();
    if c_level_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.c_level_p = c_level_pp.read();
    if world.c_level_p.is_bad_read_ptr(8) {
        return None;
    }

    // LocalClientDI
    let local_client_pp = world
        .session_cooperative_di_p
        .byte_add(0xB8)
        .cast::<*const LocalClientDI>();
    if local_client_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.local_client_di_p = local_client_pp.read();
    if world.local_client_di_p.is_bad_read_ptr(8) {
        return None;
    }

    // PlayerDI
    let player_di_pp = world
        .local_client_di_p
        .byte_add(0x50)
        .cast::<*mut PlayerDI>();
    if player_di_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.player_di_p = player_di_pp.read();
    if world.player_di_p.is_bad_read_ptr(8) {
        return None;
    }

    // PlayerPos
    world.player_world_pos_p = world.player_di_p.byte_add(0x7B0).cast::<Vec3<f32>>();

    if world.player_world_pos_p.is_bad_read_ptr(12) {
        return None;
    }
    if world.player_world_pos_p.read().y == 0.0 {
        return None;
    }

    // Angle
    world.camera_angle_p = world.player_di_p.byte_add(0x111C).cast::<Vec2<f32>>();

    if world.camera_angle_p.is_bad_read_ptr(8) {
        return None;
    }

    // PlayerCModelObject
    let player_c_model_obj_pp = world
        .player_di_p
        .byte_sub(0x50)
        .cast::<*const CModelObject>();
    if player_c_model_obj_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.player_c_model_obj_p = player_c_model_obj_pp.read();
    if world.player_c_model_obj_p.is_bad_read_ptr(8) {
        return None;
    }

    // CameraManagerDI
    let camera_manager_pp = world
        .session_cooperative_di_p
        .byte_add(0xC0)
        .cast::<*const CameraManagerDI>();
    if camera_manager_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.camera_manage_di_p = camera_manager_pp.read();
    if world.camera_manage_di_p.is_bad_read_ptr(8) {
        return None;
    }

    // CameraFPPDI
    let camera_fpp_di_pp = world
        .camera_manage_di_p
        .byte_add(0x50)
        .cast::<*const CameraFPPDI>();
    if camera_fpp_di_pp.is_bad_read_ptr(8) {
        return None;
    }
    world.camera_fpp_di_p = camera_fpp_di_pp.read();
    if world.camera_fpp_di_p.is_bad_read_ptr(8) {
        return None;
    }

    impls::get_objects_in_frustum(world.camera_fpp_di_p, &raw const WORLD_MODEL_OBJ_ARRAY, 0.0);

    Some(world)
}

#[inline(always)]
unsafe fn get_obj(model_obj_p: *const ModelObject) -> Option<Obj> {
    let mut obj = Obj {
        model_obj_p,
        ..Default::default()
    };

    let c_model_obj_pp = obj.model_obj_p.byte_add(0x20).cast::<*const CModelObject>();
    if c_model_obj_pp.is_bad_read_ptr(8) {
        return None;
    }

    // CModelObject
    obj.c_model_obj_p = c_model_obj_pp.read();
    if obj.c_model_obj_p.is_bad_read_ptr(8) {
        return None;
    }

    // Logo
    obj.c_model_obj_logo_p = obj.c_model_obj_p.byte_add(0x340).cast::<u32>();
    if obj.c_model_obj_logo_p.is_bad_read_ptr(4) {
        return None;
    }

    // 0x1 AI Preset , Shape Box, PlayerFall 等等
    // 0x2 可能是书信物件，也可能记错了
    // 0x8 可能是可互动物件，也可能记错了
    // 0x20 玩家: 人类和猎手
    // 0x40 站着的僵尸
    // 0x80 倒地的丧尸
    // 0x2000 所有NPC，包括商人
    // 0x40000 正在倒地的丧尸

    match obj.c_model_obj_logo_p.read() {
        0x0 | 0x1 | 0x2 | 0x8 => return None,
        _ => (),
    }

    let world_pos_x = obj.c_model_obj_p.byte_add(0x11C).cast::<f32>();
    let world_pos_y = obj.c_model_obj_p.byte_add(0x12C).cast::<f32>();
    let world_pos_z = obj.c_model_obj_p.byte_add(0x13C).cast::<f32>();

    if world_pos_x.is_bad_read_ptr(4)
        || world_pos_y.is_bad_read_ptr(4)
        || world_pos_z.is_bad_read_ptr(4)
    {
        return None;
    }

    obj.c_model_obj_world_pos.x = world_pos_x.read();
    obj.c_model_obj_world_pos.y = world_pos_y.read();
    obj.c_model_obj_world_pos.z = world_pos_z.read();

    // get_world_position(obj.model_obj_p, &obj.c_model_obj_world_pos);

    if obj.c_model_obj_world_pos.x == 0.0
        && obj.c_model_obj_world_pos.y == 0.0
        && obj.c_model_obj_world_pos.z == 0.0
    {
        return None;
    }

    // ModelObjectHealth
    let health_module_pp = obj
        .model_obj_p
        .byte_add(0xCE8)
        .cast::<*const HealthModule>();
    if health_module_pp.is_bad_read_ptr(8) {
        return None;
    }
    let model_health_p = health_module_pp.read();
    if model_health_p.is_bad_read_ptr(8) {
        return None;
    }

    obj.model_obj_health_p = model_health_p.byte_add(0x78).cast::<f32>().cast_mut();
    if obj.model_obj_health_p.is_bad_read_ptr(4) {
        return None;
    }
    if obj.model_obj_health_p.read() == 0.0 {
        return None;
    }

    // ModelObjectTypeData
    let model_type_data_pp = obj.c_model_obj_p.byte_add(0x60).cast::<*const i8>();
    if model_type_data_pp.is_bad_read_ptr(8) {
        return None;
    }

    obj.model_obj_str_p = model_type_data_pp.read();
    if obj.model_obj_str_p.is_bad_read_ptr(8) {
        return None;
    }

    obj.model_obj_str = std::ffi::CStr::from_ptr(obj.model_obj_str_p)
        .to_string_lossy()
        .to_string();

    let bytes = obj.model_obj_str.as_bytes();

    let start = bytes.iter().position(|&b| b == b';')? + 1;
    if start >= bytes.len() {
        return None;
    }

    match &bytes[start..] {
        // b if b.starts_with(b"Nig")
        //     || b.starts_with(b"Scr")
        //     || b.starts_with(b"Gas")
        //     || b.starts_with(b"Dem")
        //     || b.starts_with(b"Goo")
        //     || b.starts_with(b"Toa")
        //     || b.starts_with(b"Bom")
        //     // BTZ_Su BTZ_Bi
        //     || b.starts_with(b"BTZ_Su") =>
        // {
        //     obj.model_obj_type = ModelType::ZombieSpecial
        // }
        b if b.starts_with(b"Bi") || b.starts_with(b"Vi") || b.starts_with(b"Dea") => {
            obj.model_obj_type = ModelType::ZombieNormal
        }

        b if b.starts_with(b"Ni")
            || b.starts_with(b"Sc")
            || b.starts_with(b"Ga")
            || b.starts_with(b"Dem")
            || b.starts_with(b"Go")
            || b.starts_with(b"To")
            || b.starts_with(b"Bo")
            // BTZ_Su BTZ_Bi
            || b.starts_with(b"BT") =>
        {
            obj.model_obj_type = ModelType::ZombieSpecial
        }

        b if b.starts_with(b"Vo") => obj.model_obj_type = ModelType::ZombieHunter,

        b if b.starts_with(b"Zo") || b.starts_with(b"DW") => {
            // DW_Zombie
            obj.model_obj_type = ModelType::PlayerHunter
        }

        // enc很多是中立    Quest_GoodNight是友好NPC
        b if b.starts_with(b"en") || b.starts_with(b"0T") => {
            obj.model_obj_type = ModelType::SurvivorSpecial
        }

        b if b.starts_with(b"Sh") || b.starts_with(b"Sp") => {
            obj.model_obj_type = ModelType::SurvivorShopkeeper
        }

        b if b.starts_with(b"Pl") => obj.model_obj_type = ModelType::PlayerHuman,

        // 塔楼上面的坐在桌子面前操作的机械工
        // 泽雷博士车门前躺着的马里克
        // b if b.starts_with(b"Hub") || b.starts_with(b"Maa") => obj.model_obj_type = ModelType::SurvivorNormal,
        _ => {
            if obj.c_model_obj_logo_p.read() == 0x2000 {
                obj.model_obj_type = ModelType::SurvivorNormal
            } else {
                obj.model_obj_type = ModelType::Other
            }
        }
    };

    Some(obj)
}

// #[target_feature(enable = "sse")]
// unsafe  fn get_distance_to_sse(pos1: Vec3<f32>, pos2: Vec3<f32>) -> f32 {
//     let a = std::arch::x86_64::_mm_set_ps(0.0, pos1.z, pos1.y, pos1.x);
//     let b = std::arch::x86_64::_mm_set_ps(0.0, pos2.z, pos2.y, pos2.x);
//     let diff = std::arch::x86_64::_mm_sub_ps(b, a);
//     let sq = std::arch::x86_64::_mm_mul_ps(diff, diff);

//     let mut result = [0.0f32; 4];
//     std::arch::x86_64::_mm_storeu_ps(result.as_mut_ptr(), sq);
//     (result[0] + result[1] + result[2]) / 10.0
// }

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(
    h_module: isize,
    ul_reason_for_call: u32,
    _: *const core::ffi::c_void,
) -> i32 {
    if ul_reason_for_call == 1 {
        hudhook::windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls(
            hudhook::windows::Win32::Foundation::HMODULE(h_module),
        )
        .unwrap_or_default();

        spawn(move || {
            loop {
                if let Some(engine_info) = libmem::find_module("engine_x64_rwdi.dll")
                    && let Some(gamedll_info) = libmem::find_module("gamedll_x64_rwdi.dll")
                    && engine_info.base != 0
                    && gamedll_info.base != 0
                {
                    ENGINE_DLL_INFO = engine_info;
                    GAME_DLL_INFO = gamedll_info;
                    break;
                }

                std::thread::sleep(std::time::Duration::from_secs(5));
            }

            let mut game = Game::default();

            game.aim_mouse_yaw_p = 8 + libmem::sig_scan(
                "F3 0F 11 83 78 11 00 00 F3 0F 11 B3 74 11",
                GAME_DLL_INFO.base,
                GAME_DLL_INFO.size,
            )
            .unwrap();
            game.aim_mouse_pitch_p = 8 + libmem::sig_scan(
                "F3 0F 58 B3 74 11 00 00 F3 0F 11 83 78 11",
                GAME_DLL_INFO.base,
                GAME_DLL_INFO.size,
            )
            .unwrap();

            let c_game_pp_base = libmem::sig_scan(
                "48 83 EC 50 48 8B 05 ?? ?? ?? ?? 49 8B F8 48 8B",
                ENGINE_DLL_INFO.base,
                ENGINE_DLL_INFO.size,
            )
            .unwrap();

            let c_game_pp = (c_game_pp_base + 4) as *const *const CGame;

            let c_game_pp_offset = c_game_pp.byte_add(3).cast::<u32>().read_unaligned() + 7;

            CGAME_PP = c_game_pp.byte_add(c_game_pp_offset as usize).cast();

            while let None = get_world() {
                std::thread::sleep(std::time::Duration::from_secs(5));
            }

            if hudhook::Hudhook::builder()
                .with::<hudhook::hooks::dx11::ImguiDx11Hooks>(game)
                .with_hmodule(hudhook::windows::Win32::Foundation::HINSTANCE(h_module))
                .build()
                .apply()
                .is_err()
            {
                hudhook::eject();
            }
        });
    }

    1
}
