mod impls;

use hudhook::{
    imgui::{
        TabBarFlags,
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
    get_bone_joint_pos, get_distance_to, get_position, get_screen_height, get_screen_width,
    get_world_position, is_in_frustum, point_to_screen, raytest_to_target,
};
use std::{
    f32::consts::PI,
    ffi::CStr,
    ptr::{null, null_mut},
    thread::spawn,
};

trait IsBadPtr {
    unsafe fn is_bad_read_ptr(&self, size: usize) -> bool;
}

struct Ui {
    game_window: HWND,
    is_menu_on: bool,

    bone_list: [EBones; 15],
    selected_bone: EBones,

    switch_filter_zombie_normal: bool,
    switch_filter_zombie_special: bool,
    switch_filter_zombie_hunter: bool,

    switch_filter_survivor_normal: bool,
    switch_filter_survivor_special: bool,
    switch_filter_survivor_shopkeeper: bool,

    switch_filter_player_human: bool,
    switch_filter_player_hunter: bool,

    switch_filter_other: bool,

    switch_draw_model_type: bool,
    switch_draw_bones: bool,
    switch_draw_distance: bool,
    switch_draw_visible_line: bool,
    switch_draw_type_data: bool,
    switch_draw_logo: bool,

    aim: Aim,
    aim_fov: f32,
    aim_selected_key: AimKeys,
    switch_aim: bool,
    switch_draw_aim_fov: bool,
    switch_aim_key_list: [AimKeys; 5],

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

struct Dip {
    game_p: *const CGame,
    game_di_p: *const GameDI,
    session_cooperative_di_p: *const SessionCooperativeDI,
    level_di_p: *const LevelDI,
    level_p: *const CLevel,
    model_obj_p_array: Array<*const CModelObject>,
    local_client_di_p: *const LocalClientDI,
    player_di_p: *mut PlayerDI,
    player_model_obj_p: *const CModelObject,
    camera_manage_p: *const CameraManagerDI,
    camera_fpp_di_p: *const CameraFPPDI,
    player_pos_x_p: *mut f32,
    player_pos_y_p: *mut f32,
    player_pos_z_p: *mut f32,
    camera_angle_yaw_p: *mut f32,
    camera_angle_pitch_p: *mut f32,
}

struct Obj {
    c_model_obj_p: *const CModelObject,
    logo_p: *const [u8; 4],
    model_obj_p: *const ModelObject,
    pos_x_p: *const f32,
    pos_y_p: *const f32,
    pos_z_p: *const f32,
    health_p: *mut f32,
    type_data_p: *const i8,
    type_data: String,
    model_type: ModelType,
}

#[derive(Debug, Clone)]
struct Aim {
    aim_vk_code: i32,

    is_aim_key_down: bool,
    is_mouse_patched: bool,

    aiming_model_obj_p: *const ModelObject,
    best_closest_distance: f32,
    best_closest_model_obj_p: *const ModelObject,

    mouse_yaw_addr: usize,
    mouse_pitch_addr: usize,

    switch_aim_filter_zombie_normal: bool,
    switch_aim_filter_zombie_special: bool,
    switch_aim_filter_zombie_hunter: bool,
    switch_aim_filter_survivor_special: bool,
    switch_aim_filter_player_human: bool,
    switch_aim_filter_player_hunter: bool,
}

#[repr(C)]
struct Array<T> {
    ptr: *const T,
    len: u32,
    max: u32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Vec2Float {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Vec3F {
    x: f32,
    y: f32,
    z: f32,
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

#[repr(C)]
#[derive(Default, PartialEq)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
#[repr(i32)]
enum AimKeys {
    #[default]
    RMouseButton = 0x2,
    LCtrl = 0xA2,
    LShift = 0xA0,
    LAlt = 0xA4,
    F = 0x46,
}

impl<T> IsBadPtr for *const T {
    unsafe fn is_bad_read_ptr(&self, size: usize) -> bool {
        IsBadReadPtr(Some(self.cast()), size).as_bool()
    }
}

impl<T> IsBadPtr for *mut T {
    unsafe fn is_bad_read_ptr(&self, size: usize) -> bool {
        IsBadReadPtr(Some(self.cast()), size).as_bool()
    }
}

impl std::fmt::Display for EBones {
    #[inline(always)]
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

impl std::fmt::Display for AimKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AimKeys::F => write!(f, "F"),
            AimKeys::LShift => write!(f, "左Shift"),
            AimKeys::LAlt => write!(f, "左Alt"),
            AimKeys::RMouseButton => write!(f, "鼠标右"),
            AimKeys::LCtrl => write!(f, "左Ctrl"),
        }
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::ZombieNormal => write!(f, "丧尸(普通)"),
            ModelType::ZombieSpecial => write!(f, "丧尸(特殊)"),
            ModelType::ZombieHunter => write!(f, "丧尸(猎手)"),
            ModelType::SurvivorNormal => write!(f, "幸存者(普通)"),
            ModelType::SurvivorShopkeeper => write!(f, "幸存者(商人)"),
            ModelType::SurvivorSpecial => write!(f, "幸存者(特殊)"),
            ModelType::PlayerHuman => write!(f, "玩家(人类)"),
            ModelType::PlayerHunter => write!(f, "玩家(猎手)"),
            ModelType::Other => write!(f, "其他(调试)"),
        }
    }
}

unsafe impl Send for Ui {}

unsafe impl Sync for Ui {}

impl hudhook::ImguiRenderLoop for Ui {
    unsafe fn initialize<'a>(
        &'a mut self,
        ctx: &mut hudhook::imgui::Context,
        _: &'a mut dyn hudhook::RenderContext,
    ) {
        self.game_window = FindWindowA(hudhook::windows::core::s!("techland_game_class"), None);

        ImFontAtlas_AddFontFromFileTTF(
            ctx.fonts().raw_mut(),
            "C:\\windows\\fonts\\simhei.ttf\0".as_ptr().cast(),
            22.0,
            std::ptr::null(),
            ImFontAtlas_GetGlyphRangesChineseFull(ctx.fonts().raw_mut()),
        );

        ctx.style_mut().use_light_colors();

        ctx.set_ini_filename(None);
    }

    unsafe fn before_render<'a>(
        &'a mut self,
        ctx: &mut hudhook::imgui::Context,
        _: &'a mut dyn hudhook::RenderContext,
    ) {
        static mut IS_KEY_OPEN_MENU_DOWN: bool = false;

        if GetAsyncKeyState(0xC0) & 0x8000u16 as i16 != 0 {
            if !IS_KEY_OPEN_MENU_DOWN {
                IS_KEY_OPEN_MENU_DOWN = true;
                self.is_menu_on = !self.is_menu_on;
            }
        } else if IS_KEY_OPEN_MENU_DOWN {
            IS_KEY_OPEN_MENU_DOWN = false;
        }

        if !self.is_menu_on {
            ctx.io_mut().mouse_draw_cursor = false;
            return;
        }

        let mut mouse_pos: hudhook::windows::Win32::Foundation::POINT =
            hudhook::windows::Win32::Foundation::POINT {
                x: 0,
                y: 0,
            };

        let _ = GetCursorPos(&mut mouse_pos);
        let _ = ScreenToClient(self.game_window, &mut mouse_pos);

        ctx.io_mut().mouse_draw_cursor = true;
        ctx.io_mut().mouse_pos[0] = mouse_pos.x as f32;
        ctx.io_mut().mouse_pos[1] = mouse_pos.y as f32;

        static mut IS_MOUSE_LEFT_DOWN: bool = false;

        if GetAsyncKeyState(0x1) & 0x8000u16 as i16 != 0 {
            IS_MOUSE_LEFT_DOWN = true;

            ctx.io_mut().mouse_down[0] = true;
        } else {
            IS_MOUSE_LEFT_DOWN = false;

            ctx.io_mut().mouse_down[0] = false;
        }
    }

    unsafe fn render(&mut self, ui: &mut hudhook::imgui::Ui) {
        let dip = match get_world() {
            Some(val) => val,
            None => return,
        };

        if self.switch_draw_aim_fov {
            self.draw_aim_fov(ui, &dip);
        }

        for index in 0..dip.model_obj_p_array.len {
            let c_model_obj_pp = dip.model_obj_p_array.ptr.add(index as usize);
            if c_model_obj_pp.is_bad_read_ptr(size_of::<*const *const CModelObject>()) {
                continue;
            }

            let obj = match get_obj(c_model_obj_pp.read()) {
                Some(val) => val,
                None => continue,
            };

            if obj.c_model_obj_p == dip.player_model_obj_p {
                continue;
            }

            let (filter, color) = match obj.model_type {
                ModelType::ZombieNormal => {
                    (self.switch_filter_zombie_normal, self.color_zombie_normal)
                }
                ModelType::ZombieSpecial => {
                    (self.switch_filter_zombie_special, self.color_zombie_special)
                }
                ModelType::ZombieHunter => {
                    (self.switch_filter_zombie_hunter, self.color_zombie_hunter)
                }
                ModelType::SurvivorNormal => {
                    (
                        self.switch_filter_survivor_normal,
                        self.color_survivor_nomal,
                    )
                }
                ModelType::SurvivorSpecial => {
                    (
                        self.switch_filter_survivor_special,
                        self.color_survivor_special,
                    )
                }
                ModelType::SurvivorShopkeeper => {
                    (
                        self.switch_filter_survivor_shopkeeper,
                        self.color_survivor_shopkeeper,
                    )
                }
                ModelType::PlayerHuman => {
                    (self.switch_filter_player_human, self.color_player_human)
                }
                ModelType::PlayerHunter => {
                    (self.switch_filter_player_hunter, self.color_player_hunter)
                }
                ModelType::Other => (self.switch_filter_other, self.color_other),
            };

            if !filter {
                continue;
            }

            // let mut world_pos = Vec3Float::default();
            // get_bone_joint_pos(
            //     obj.model_obj_p,
            //     &mut world_pos,
            //     self.selected_bone as u8,
            // );

            let mut world_pos: Vec3F = Vec3F::default();
            get_world_position(obj.model_obj_p, &mut world_pos);

            if is_in_frustum(dip.camera_fpp_di_p, &mut world_pos) == 0 {
                continue;
            }

            if self.switch_aim {
                self.aim.update_obj(&dip, &obj, &world_pos, self.aim_fov);
            }

            if self.switch_draw_model_type {
                self.draw_model_type(ui, &dip, &obj, color, &world_pos);
            }

            if self.switch_draw_bones {
                self.draw_bones(ui, &dip, &obj, color);
            }

            if self.switch_draw_distance {
                self.draw_distance(ui, &dip, &obj);
            }

            if self.switch_draw_visible_line {
                self.draw_visible_line(ui, &dip, &obj, color, &world_pos);
            }

            if self.switch_draw_type_data {
                self.draw_type_data(ui, &dip, &obj, color, &world_pos);
            }

            if self.switch_draw_logo {
                self.draw_logo(ui, &dip, &obj, color, &world_pos);
            }
        }

        if self.switch_aim {
            self.aim.obj_lock(&dip, self.selected_bone as u8);
        }

        if self.is_menu_on {
            ui.window("[~]键")
                .title_bar(true)
                .size([600.0, 450.0], hudhook::imgui::Condition::FirstUseEver)
                .build(|| {
                    if let Some(bar) = ui
                        .tab_bar_with_flags("##tab_bar", TabBarFlags::NO_TAB_LIST_SCROLLING_BUTTONS)
                    {
                        self.tab_item_filter(ui);

                        self.tab_item_draw(ui);

                        self.tab_item_aim(ui);

                        bar.end();
                    }
                });
        }
    }
}

impl Ui {
    const BONE_LISTS: &'static [&'static [EBones]] = &[
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

    unsafe fn tab_item_filter(&mut self, ui: &hudhook::imgui::Ui) {
        let Some(item) = ui.tab_item("过滤") else {
            return;
        };

        // ZombieNormal
        ui.checkbox(
            "丧尸(普通)##switch_filter_zombie_normal",
            &mut self.switch_filter_zombie_normal,
        );
        ui.same_line();
        ui.color_edit4_config("##color_zombie_normal", &mut self.color_zombie_normal)
            .inputs(false)
            .build();

        // ZombieSpecial
        ui.checkbox(
            "丧尸(特殊)##switch_filter_zombie_special",
            &mut self.switch_filter_zombie_special,
        );
        ui.same_line();
        ui.color_edit4_config("##color_zombie_special", &mut self.color_zombie_special)
            .inputs(false)
            .build();

        // ZombieHunter
        ui.checkbox(
            "丧尸(猎手)##switch_filter_zombie_hunter",
            &mut self.switch_filter_zombie_hunter,
        );
        ui.same_line();
        ui.color_edit4_config("##color_zombie_hunter", &mut self.color_zombie_hunter)
            .inputs(false)
            .build();

        // SurvivorNormal
        ui.checkbox(
            "幸存者(普通)##switch_filter_survivor_normal",
            &mut self.switch_filter_survivor_normal,
        );
        ui.same_line();
        ui.color_edit4_config("##color_survivor_nomal", &mut self.color_survivor_nomal)
            .inputs(false)
            .build();

        // SurvivorSpecial
        ui.checkbox(
            "幸存者(特殊)##switch_filter_survivor_special,",
            &mut self.switch_filter_survivor_special,
        );
        ui.same_line();
        ui.color_edit4_config("##color_survivor_special", &mut self.color_survivor_special)
            .inputs(false)
            .build();

        // SurvivorShopkeeper
        ui.checkbox(
            "幸存者(商人)##switch_filter_survivor_shopkeeper",
            &mut self.switch_filter_survivor_shopkeeper,
        );
        ui.same_line();
        ui.color_edit4_config(
            "##color_survivor_shopkeeper",
            &mut self.color_survivor_shopkeeper,
        )
        .inputs(false)
        .build();

        // PlayerHuman
        ui.checkbox(
            "玩家(人类)##switch_filter_player_human",
            &mut self.switch_filter_player_human,
        );
        ui.same_line();
        ui.color_edit4_config("##color_player_human", &mut self.color_player_human)
            .inputs(false)
            .build();

        // PlayerHunter
        ui.checkbox(
            "玩家(猎手)##switch_filter_player_hunter",
            &mut self.switch_filter_player_hunter,
        );
        ui.same_line();
        ui.color_edit4_config("##color_player_hunter", &mut self.color_player_hunter)
            .inputs(false)
            .build();

        // Other
        ui.checkbox(
            "其他(调试)##witch_filter_other",
            &mut self.switch_filter_other,
        );
        ui.same_line();
        ui.color_edit4_config("##color_other", &mut self.color_other)
            .inputs(false)
            .build();

        item.end();
    }

    unsafe fn tab_item_draw(&mut self, ui: &hudhook::imgui::Ui) {
        let Some(item) = ui.tab_item("绘制") else {
            return;
        };
        ui.checkbox("名字", &mut self.switch_draw_model_type);

        ui.checkbox("骨骼", &mut self.switch_draw_bones);

        ui.checkbox("距离", &mut self.switch_draw_distance);

        ui.checkbox("可视线", &mut self.switch_draw_visible_line);

        ui.checkbox("类型", &mut self.switch_draw_type_data);

        ui.checkbox("标志", &mut self.switch_draw_logo);

        item.end();
    }

    unsafe fn tab_item_aim(&mut self, ui: &hudhook::imgui::Ui) {
        let Some(item) = ui.tab_item("自瞄") else {
            return;
        };

        ui.checkbox("开启##switch_aim", &mut self.switch_aim);

        if let Some(cb) =
            ui.begin_combo("按键##aim_selected_key", self.aim_selected_key.to_string())
        {
            for current in self.switch_aim_key_list.as_slice() {
                if self.aim_selected_key == *current {
                    ui.set_item_default_focus();
                }

                if ui
                    .selectable_config(current.to_string())
                    .selected(self.aim_selected_key == *current)
                    .build()
                {
                    self.aim_selected_key = *current;
                    self.aim.aim_vk_code = self.aim_selected_key as i32;
                }
            }
            cb.end();
        }

        if let Some(cb) = ui.begin_combo("部位", self.selected_bone.to_string()) {
            for current in self.bone_list.as_slice() {
                if self.selected_bone == *current {
                    ui.set_item_default_focus();
                }

                if ui
                    .selectable_config(current.to_string())
                    .selected(self.selected_bone == *current)
                    .build()
                {
                    self.selected_bone = *current;
                }
            }
            cb.end();
        }

        ui.checkbox("FOV##switch_aim_fov", &mut self.switch_draw_aim_fov);
        ui.same_line();
        ui.slider("##aim_fov", 50.0, 500.0, &mut self.aim_fov);

        ui.checkbox(
            "丧尸(普通)##switch_aim_filter_zombie_normal",
            &mut self.aim.switch_aim_filter_zombie_normal,
        );

        ui.checkbox(
            "丧尸(特殊)##switch_aim_filter_zombie_special",
            &mut self.aim.switch_aim_filter_zombie_special,
        );

        ui.checkbox(
            "丧尸(猎手)##switch_aim_filter_survivor_special",
            &mut self.aim.switch_aim_filter_zombie_hunter,
        );

        ui.checkbox(
            "幸存者(特殊)##switch_aim_filter_survivor_special",
            &mut self.aim.switch_aim_filter_survivor_special,
        );

        ui.checkbox(
            "玩家(人类)##switch_aim_filter_player_human",
            &mut self.aim.switch_aim_filter_player_human,
        );

        ui.checkbox(
            "玩家(猎手)##switch_aim_filter_player_hunter",
            &mut self.aim.switch_aim_filter_player_hunter,
        );

        item.end();
    }

    #[inline(always)]
    unsafe fn draw_model_type(
        &self,
        ui: &hudhook::imgui::Ui,
        dip: &Dip,
        obj: &Obj,
        color: [f32; 4],
        world_pos: &Vec3F,
    ) {
        let mut screen_pos: Vec2Float = Vec2Float::default();

        point_to_screen(dip.camera_fpp_di_p, &mut screen_pos, world_pos);

        ui.get_background_draw_list().add_text(
            [screen_pos.x, screen_pos.y],
            color,
            obj.model_type.to_string(),
        );
    }

    #[inline(always)]
    unsafe fn draw_bones(&self, ui: &hudhook::imgui::Ui, dip: &Dip, obj: &Obj, color: [f32; 4]) {
        let mut previous_world_pos: Vec3F = Vec3F {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        let mut current_world_pos: Vec3F = Vec3F {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        for bone_list in Self::BONE_LISTS {
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

                let mut previous_screen_pos: Vec2Float = Vec2Float {
                    x: 0.0,
                    y: 0.0,
                };

                let mut current_screen_pos: Vec2Float = Vec2Float {
                    x: 0.0,
                    y: 0.0,
                };

                point_to_screen(
                    dip.camera_fpp_di_p,
                    &mut previous_screen_pos,
                    &mut previous_world_pos,
                );

                point_to_screen(
                    dip.camera_fpp_di_p,
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
    unsafe fn draw_distance(&self, ui: &hudhook::imgui::Ui, dip: &Dip, obj: &Obj) {
        let mut screen_pos: Vec2Float = Vec2Float::default();

        let mut world_pos = Vec3F {
            x: obj.pos_x_p.read(),
            y: obj.pos_y_p.read(),
            z: obj.pos_z_p.read(),
        };

        point_to_screen(dip.camera_fpp_di_p, &mut screen_pos, &mut world_pos);

        let player_world_pos = Vec3F {
            x: dip.player_pos_x_p.read(),
            y: dip.player_pos_y_p.read(),
            z: dip.player_pos_z_p.read(),
        };

        ui.get_background_draw_list().add_text(
            [screen_pos.x, screen_pos.y],
            self.color_zombie_normal,
            get_distance_to(obj.model_obj_p, &player_world_pos).to_string(),
        );
    }

    #[inline(always)]
    unsafe fn draw_visible_line(
        &self,
        ui: &hudhook::imgui::Ui,
        dip: &Dip,
        obj: &Obj,
        color: [f32; 4],
        world_pos: &Vec3F,
    ) {
        let mut screen_pos: Vec2Float = Vec2Float {
            x: 0.0,
            y: 0.0,
        };

        point_to_screen(dip.camera_fpp_di_p, &mut screen_pos, world_pos);

        if raytest_to_target(
            obj.model_obj_p,
            get_position(dip.camera_fpp_di_p),
            world_pos,
        ) == 0
        {
            return;
        }

        ui.get_background_draw_list()
            .add_line(
                [
                    get_screen_width(dip.game_di_p) as f32 / 2.0,
                    get_screen_height(dip.game_di_p) as f32,
                ],
                [screen_pos.x, screen_pos.y],
                color,
            )
            .thickness(2.5)
            .build();
    }

    #[inline(always)]
    unsafe fn draw_type_data(
        &self,
        ui: &hudhook::imgui::Ui,
        dip: &Dip,
        obj: &Obj,
        color: [f32; 4],
        world_pos: &Vec3F,
    ) {
        let mut screen_pos: Vec2Float = Vec2Float::default();

        point_to_screen(dip.camera_fpp_di_p, &mut screen_pos, world_pos);

        ui.get_background_draw_list().add_text(
            [screen_pos.x, screen_pos.y],
            color,
            obj.type_data.as_str(),
        );
    }

    #[inline(always)]
    unsafe fn draw_logo(
        &self,
        ui: &hudhook::imgui::Ui,
        dip: &Dip,
        obj: &Obj,
        color: [f32; 4],
        world_pos: &Vec3F,
    ) {
        let mut screen_pos: Vec2Float = Vec2Float::default();

        point_to_screen(dip.camera_fpp_di_p, &mut screen_pos, world_pos);

        ui.get_background_draw_list().add_text(
            [screen_pos.x, screen_pos.y],
            color,
            format!("{:#X?}", obj.logo_p.read()),
        );
    }

    #[inline(always)]
    unsafe fn draw_aim_fov(&self, ui: &hudhook::imgui::Ui, dip: &Dip) {
        ui.get_background_draw_list()
            .add_circle(
                [
                    get_screen_width(dip.game_di_p) as f32 / 2.0,
                    get_screen_height(dip.game_di_p) as f32 / 2.0,
                ],
                self.aim_fov,
                self.color_zombie_normal,
            )
            .build();
    }
}

impl Aim {
    const NOP_8: [u8; 8] = [0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90];
    const PITCH_ORIGINAL: [u8; 8] = [0xF3, 0x0F, 0x11, 0x83, 0x78, 0x11, 0x00, 0x00];
    const YAW_ORIGINAL: [u8; 8] = [0xF3, 0x0F, 0x11, 0xB3, 0x74, 0x11, 0x00, 0x00];

    #[inline(always)]
    unsafe fn update_obj(&mut self, dip: &Dip, obj: &Obj, world_pos: &Vec3F, aim_fov: f32) {
        if !match obj.model_type {
            ModelType::Other | ModelType::SurvivorNormal | ModelType::SurvivorShopkeeper => return,
            ModelType::ZombieNormal => self.switch_aim_filter_zombie_normal,
            ModelType::ZombieSpecial => self.switch_aim_filter_zombie_special,
            ModelType::ZombieHunter => self.switch_aim_filter_zombie_hunter,
            ModelType::SurvivorSpecial => self.switch_aim_filter_survivor_special,
            ModelType::PlayerHuman => self.switch_aim_filter_player_human,
            ModelType::PlayerHunter => self.switch_aim_filter_player_hunter,
        } {
            return;
        }

        let mut screen_pos = Vec2Float::default();

        point_to_screen(dip.camera_fpp_di_p, &mut screen_pos, world_pos);

        let pow_x = (screen_pos.x - get_screen_width(dip.game_di_p) as f32 / 2.0).powf(2.0);

        let pow_y = (screen_pos.y - get_screen_height(dip.game_di_p) as f32 / 2.0).powf(2.0);

        let distance = (pow_x + pow_y).sqrt();

        if distance > aim_fov || distance > self.best_closest_distance {
            return;
        }

        self.best_closest_distance = distance;
        self.best_closest_model_obj_p = obj.model_obj_p;
    }

    #[inline(always)]
    unsafe fn obj_lock(&mut self, dip: &Dip, selected_bone: u8) {
        if !self.is_aim_key_down {
            if self.is_mouse_patched {
                self.is_mouse_patched = false;
                self.mouse_unpatch();
            }

            if self.best_closest_distance == f32::MAX {
                return;
            }
        }

        if GetAsyncKeyState(self.aim_vk_code) & 0x8000u16 as i16 != 0 {
            if !self.is_aim_key_down {
                self.is_aim_key_down = true;

                self.aiming_model_obj_p = self.best_closest_model_obj_p;

                if !self.is_mouse_patched {
                    self.is_mouse_patched = true;
                    self.mouse_patch();
                }
            }

            if self
                .aiming_model_obj_p
                .is_bad_read_ptr(size_of::<*const ModelObject>())
            {
                self.is_aim_key_down = false;

                if self.is_mouse_patched {
                    self.is_mouse_patched = false;
                    self.mouse_unpatch();
                }

                return;
            }

            let c_model_obj_pp = self
                .aiming_model_obj_p
                .byte_add(0x8)
                .cast::<*const CModelObject>();

            if c_model_obj_pp.is_bad_read_ptr(size_of::<*const *const CModelObject>()) {
                self.is_aim_key_down = false;

                if self.is_mouse_patched {
                    self.is_mouse_patched = false;
                    self.mouse_unpatch();
                }

                return;
            }

            if get_obj(c_model_obj_pp.read()).is_none() {
                self.is_aim_key_down = false;

                if self.is_mouse_patched {
                    self.is_mouse_patched = false;
                    self.mouse_unpatch();
                }

                return;
            }

            let mut world_pos: Vec3F = Vec3F {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            };

            get_bone_joint_pos(self.aiming_model_obj_p, &mut world_pos, selected_bone);

            let mut pos: Vec3F = Vec3F {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            };

            let camera_world_pos_p = get_position(dip.camera_fpp_di_p);

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

            dip.camera_angle_yaw_p.write(yaw);
            dip.camera_angle_pitch_p.write(pitch);
        } else {
            self.is_aim_key_down = false;

            if self.is_mouse_patched {
                self.is_mouse_patched = false;
                self.mouse_unpatch();
            }
        }

        self.best_closest_distance = f32::MAX;
    }

    #[inline(always)]
    unsafe fn mouse_patch(&mut self) {
        libmem::memory::write_memory_ex(
            &libmem::process::get_process().unwrap(),
            self.mouse_yaw_addr,
            Self::NOP_8.as_slice(),
        );

        libmem::memory::write_memory_ex(
            &libmem::process::get_process().unwrap(),
            self.mouse_pitch_addr,
            Self::NOP_8.as_slice(),
        );
    }

    #[inline(always)]
    unsafe fn mouse_unpatch(&mut self) {
        libmem::memory::write_memory_ex(
            &libmem::process::get_process().unwrap(),
            self.mouse_yaw_addr,
            Self::YAW_ORIGINAL.as_slice(),
        );

        libmem::memory::write_memory_ex(
            &libmem::process::get_process().unwrap(),
            self.mouse_pitch_addr,
            Self::PITCH_ORIGINAL.as_slice(),
        );
    }
}

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

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(
    h_module: isize,
    ul_reason_for_call: u32,
    _: *const core::ffi::c_void,
) -> i32 {
    if ul_reason_for_call == 1 {
        let _ = hudhook::windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls(
            hudhook::windows::Win32::Foundation::HMODULE(h_module),
        );

        spawn(move || {
            loop {
                if let (Some(engine_info), Some(gamedll_info)) = (
                    libmem::find_module("engine_x64_rwdi.dll"),
                    libmem::find_module("gamedll_x64_rwdi.dll"),
                ) {
                    if engine_info.base != 0 && gamedll_info.base != 0 {
                        ENGINE_DLL_INFO = engine_info;
                        GAME_DLL_INFO = gamedll_info;
                        break;
                    }
                }

                std::thread::sleep(std::time::Duration::from_secs(5));
            }

            let mut ui = Ui {
                aim: Aim {
                    aim_vk_code: 0x2,

                    is_aim_key_down: false,

                    is_mouse_patched: false,

                    aiming_model_obj_p: null(),

                    best_closest_distance: f32::MAX,
                    best_closest_model_obj_p: null(),

                    mouse_yaw_addr: 0,
                    mouse_pitch_addr: 0,

                    switch_aim_filter_zombie_normal: false,
                    switch_aim_filter_zombie_special: false,
                    switch_aim_filter_survivor_special: false,
                    switch_aim_filter_zombie_hunter: false,
                    switch_aim_filter_player_human: false,
                    switch_aim_filter_player_hunter: false,
                },

                game_window: HWND(0),
                is_menu_on: true,
                bone_list: [
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
                ],
                selected_bone: EBones::Head,

                switch_filter_zombie_normal: false,
                switch_filter_zombie_special: false,
                switch_filter_zombie_hunter: false,
                switch_filter_survivor_normal: false,
                switch_filter_survivor_special: false,
                switch_filter_survivor_shopkeeper: false,
                switch_filter_player_human: false,
                switch_filter_player_hunter: false,
                switch_filter_other: false,

                switch_draw_distance: false,
                switch_draw_bones: false,
                switch_draw_visible_line: false,
                switch_draw_model_type: false,
                switch_draw_type_data: false,
                switch_draw_logo: false,

                switch_aim: false,
                switch_draw_aim_fov: false,
                aim_fov: 150.0,
                switch_aim_key_list: [
                    AimKeys::RMouseButton,
                    AimKeys::LCtrl,
                    AimKeys::LShift,
                    AimKeys::LAlt,
                    AimKeys::F,
                ],
                aim_selected_key: AimKeys::RMouseButton,

                color_zombie_normal: [1.0, 1.0, 1.0, 1.0], // 白色
                color_zombie_special: [0.9375, 0.7969, 1.0, 1.0], // 粉色
                color_zombie_hunter: [1.0, 0.0, 1.0, 1.0], // 紫红色

                color_survivor_nomal: [0.0, 0.0, 1.0, 1.0], // 蓝色
                color_survivor_special: [1.0, 1.0, 0.0, 1.0], // 黄色
                color_survivor_shopkeeper: [0.0, 1.0, 1.0, 1.0], // 青色

                color_player_human: [0.0, 1.0, 0.0, 1.0], // 绿色
                color_player_hunter: [1.0, 0.0, 0.0, 1.0], // 红色
                color_other: [1.0, 1.0, 1.0, 1.0],        // 白色
            };

            init(&mut ui);

            while let None = get_world() {
                std::thread::sleep(std::time::Duration::from_secs(5));
            }

            if hudhook::Hudhook::builder()
                .with::<hudhook::hooks::dx11::ImguiDx11Hooks>(ui)
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

unsafe fn init(ui: &mut Ui) -> Option<()> {
    let c_game_pp_base = libmem::sig_scan(
        "48 83 EC 50 48 8B 05 ?? ?? ?? ?? 49 8B F8 48 8B",
        ENGINE_DLL_INFO.base,
        ENGINE_DLL_INFO.size,
    )?;

    let c_game_pp = (c_game_pp_base + 4) as *const *const CGame;

    let c_game_pp_offset = c_game_pp.byte_add(3).cast::<u32>().read_unaligned() + 7;

    CGAME_PP = c_game_pp.byte_add(c_game_pp_offset as usize).cast();

    ui.aim.mouse_yaw_addr = 8 + libmem::sig_scan(
        "F3 0F 11 83 78 11 00 00 F3 0F 11 B3 74 11",
        GAME_DLL_INFO.base,
        GAME_DLL_INFO.size,
    )?;
    ui.aim.mouse_pitch_addr = 8 + libmem::sig_scan(
        "F3 0F 58 B3 74 11 00 00 F3 0F 11 83 78 11",
        GAME_DLL_INFO.base,
        GAME_DLL_INFO.size,
    )?;

    Some(())
}

#[inline(always)]
unsafe fn get_world() -> Option<Dip> {
    let mut dip = Dip {
        game_p: null(),
        game_di_p: null(),
        session_cooperative_di_p: null(),
        level_di_p: null(),
        level_p: null(),
        model_obj_p_array: Array {
            ptr: null(),
            len: 0,
            max: 0,
        },
        local_client_di_p: null(),
        player_di_p: null_mut(),
        player_model_obj_p: null(),

        camera_manage_p: null(),
        camera_fpp_di_p: null(),

        player_pos_x_p: null_mut(),
        player_pos_y_p: null_mut(),
        player_pos_z_p: null_mut(),
        camera_angle_yaw_p: null_mut(),
        camera_angle_pitch_p: null_mut(),
    };

    // CGame
    dip.game_p = CGAME_PP.read();

    // GameDI
    let game_di_pp = dip.game_p.byte_add(0x98).cast::<*const GameDI>();
    if game_di_pp.is_null() {
        return None;
    }
    dip.game_di_p = game_di_pp.read();
    if dip.game_di_p.is_null() {
        return None;
    }

    // SessionCooperativeDI
    let session_cooperative_di_pp = dip
        .game_di_p
        .byte_add(0x540)
        .cast::<*const SessionCooperativeDI>();
    if session_cooperative_di_pp.is_null() {
        return None;
    }
    dip.session_cooperative_di_p = session_cooperative_di_pp.read();
    if dip.session_cooperative_di_p.is_null() {
        return None;
    }

    // LevelDI
    let level_di_pp = dip
        .session_cooperative_di_p
        .byte_add(0xB0)
        .cast::<*const LevelDI>();
    if level_di_pp.is_null() {
        return None;
    }
    dip.level_di_p = level_di_pp.read();
    if dip.level_di_p.is_null() {
        return None;
    }

    // CLevel
    let c_level_pp = dip.level_di_p.byte_add(0x8).cast::<*const CLevel>();
    if c_level_pp.is_null() {
        return None;
    }
    dip.level_p = c_level_pp.read();
    if dip.level_p.is_null() {
        return None;
    }

    // Array: CModelObject
    let c_model_obj_p_array_p = dip
        .level_p
        .byte_add(0x928)
        .cast::<Array<*const CModelObject>>();
    if c_model_obj_p_array_p.is_null() {
        return None;
    }
    dip.model_obj_p_array = c_model_obj_p_array_p.read();
    if dip.model_obj_p_array.ptr.is_null() {
        return None;
    }
    if dip.model_obj_p_array.len == 0 {
        return None;
    }

    // LocalClientDI
    let local_client_pp = dip
        .session_cooperative_di_p
        .byte_add(0xB8)
        .cast::<*const LocalClientDI>();
    if local_client_pp.is_null() {
        return None;
    }
    dip.local_client_di_p = local_client_pp.read();
    if dip.local_client_di_p.is_null() {
        return None;
    }

    // PlayerDI
    let player_di_pp = dip.local_client_di_p.byte_add(0x50).cast::<*mut PlayerDI>();
    if player_di_pp.is_null() {
        return None;
    }
    dip.player_di_p = player_di_pp.read();
    if dip.player_di_p.is_null() {
        return None;
    }

    // PlayerPos
    dip.player_pos_x_p = dip.player_di_p.byte_add(0x7B0).cast::<f32>();
    dip.player_pos_y_p = dip.player_pos_x_p.byte_add(0x4);
    dip.player_pos_z_p = dip.player_pos_y_p.byte_add(0x4);
    if dip.player_pos_x_p.is_null() || dip.player_pos_y_p.is_null() || dip.player_pos_z_p.is_null()
    {
        return None;
    }
    if dip.player_pos_x_p.read() == 0.0
        && dip.player_pos_y_p.read() == 0.0
        && dip.player_pos_z_p.read() == 0.0
    {
        return None;
    }

    // Angle
    dip.camera_angle_yaw_p = dip.player_di_p.byte_add(0x111C).cast::<f32>();
    dip.camera_angle_pitch_p = dip.camera_angle_yaw_p.byte_add(0x4);
    if dip.camera_angle_yaw_p.is_null() || dip.camera_angle_pitch_p.is_null() {
        return None;
    }

    // PlayerCModelObject
    let player_c_model_obj_pp = dip.player_di_p.byte_sub(0x50).cast::<*const CModelObject>();
    if player_c_model_obj_pp.is_bad_read_ptr(size_of::<*mut *const CModelObject>()) {
        return None;
    }
    dip.player_model_obj_p = player_c_model_obj_pp.read();
    if dip
        .player_model_obj_p
        .is_bad_read_ptr(size_of::<*const CModelObject>())
    {
        return None;
    }

    // CameraManagerDI
    let camera_manager_pp = dip
        .session_cooperative_di_p
        .byte_add(0xC0)
        .cast::<*const CameraManagerDI>();
    if camera_manager_pp.is_null() {
        return None;
    }
    dip.camera_manage_p = camera_manager_pp.read();
    if dip.camera_manage_p.is_null() {
        return None;
    }

    // CameraFPPDI
    let camera_fpp_di_pp = dip
        .camera_manage_p
        .byte_add(0x50)
        .cast::<*const CameraFPPDI>();
    if camera_fpp_di_pp.is_null() {
        return None;
    }
    dip.camera_fpp_di_p = camera_fpp_di_pp.read();
    if dip.camera_fpp_di_p.is_null() {
        return None;
    }

    Some(dip)
}

#[inline(always)]
unsafe fn get_obj(c_model_obj_p: *const CModelObject) -> Option<Obj> {
    let mut obj = Obj {
        model_obj_p: null(),
        pos_x_p: null(),
        pos_y_p: null(),
        pos_z_p: null(),
        type_data_p: null(),
        logo_p: null(),
        c_model_obj_p: null(),
        model_type: ModelType::default(),
        health_p: null_mut(),
        type_data: String::new(),
    };

    // CModelObject
    obj.c_model_obj_p = c_model_obj_p;
    if obj
        .c_model_obj_p
        .is_bad_read_ptr(size_of::<*const CModelObject>())
    {
        return None;
    }

    // Logo
    obj.logo_p = obj.c_model_obj_p.byte_add(0x340).cast::<[u8; 4]>();
    if obj.logo_p.is_bad_read_ptr(size_of::<[u8; 4]>()) {
        return None;
    }
    let logo = obj.logo_p.read();
    if i32::from_le_bytes(logo) == 0 {
        return None;
    }

    // [0] 0x1 会导致闪退
    // [0] 0x2 可能是书信物件，也可能记错了
    // [0] 0x8 可能是可互动物件，也可能记错了
    // [0] 0x20 玩家: 人类和猎手
    // [0] 0x40 僵尸
    // [0] 0x80 倒地的僵尸

    // [1] 0x20 所有NPC，包括商人

    match logo[0] {
        0x1 | 0x2 | 0x8 | 0x10 | 0x12 => return None,
        _ => (),
    }

    match logo[1] {
        0x1 | 0x8 => return None,
        _ => (),
    }

    // match logo[2] {
    //     0x8 | 0x20 => return None,
    //     _ => (),
    // }

    // ModelObject
    let model_obj_pp = obj
        .c_model_obj_p
        .byte_add(0x3B8)
        .cast::<*const ModelObject>();
    if model_obj_pp.is_bad_read_ptr(size_of::<*const *const ModelObject>()) {
        return None;
    }
    obj.model_obj_p = model_obj_pp.read();
    if obj
        .c_model_obj_p
        .is_bad_read_ptr(size_of::<*const CModelObject>())
    {
        return None;
    }

    // ModelObjectPos
    obj.pos_x_p = obj.c_model_obj_p.byte_add(0x11C).cast::<f32>();
    obj.pos_y_p = obj.c_model_obj_p.byte_add(0x12C).cast::<f32>();
    obj.pos_z_p = obj.c_model_obj_p.byte_add(0x13C).cast::<f32>();
    if obj.pos_x_p.is_bad_read_ptr(size_of::<f32>())
        || obj.pos_y_p.is_bad_read_ptr(size_of::<f32>())
        || obj.pos_z_p.is_bad_read_ptr(size_of::<f32>())
    {
        return None;
    }
    if obj.pos_x_p.read() == 0.0 && obj.pos_y_p.read() == 0.0 && obj.pos_z_p.read() == 0.0 {
        return None;
    }

    // ModelObjectHealth
    let health_module_pp = obj
        .model_obj_p
        .byte_add(0xCE8)
        .cast::<*const HealthModule>();
    if health_module_pp.is_bad_read_ptr(size_of::<*const *const HealthModule>()) {
        return None;
    }
    let model_health_p = health_module_pp.read();
    if model_health_p.is_bad_read_ptr(size_of::<*const *const HealthModule>()) {
        return None;
    }
    obj.health_p = model_health_p.byte_add(0x78).cast::<f32>().cast_mut();
    if obj.health_p.is_bad_read_ptr(size_of::<f32>()) {
        return None;
    }
    if obj.health_p.read() == 0.0 {
        return None;
    }

    // ModelObjectTypeData
    let mode_type_data_pp = obj.c_model_obj_p.byte_add(0x60).cast::<*const i8>();
    if mode_type_data_pp.is_bad_read_ptr(size_of::<*const *const i8>()) {
        return None;
    }
    obj.type_data_p = mode_type_data_pp.read();
    if obj.type_data_p.is_bad_read_ptr(size_of::<*const i8>()) {
        return None;
    }
    obj.type_data = CStr::from_ptr(obj.type_data_p).to_str().ok()?.to_string();

    let type_data_slice: Vec<&str> = obj.type_data.split(";").collect();

    if type_data_slice.len() < 2 {
        return None;
    }

    match type_data_slice[1] {
        player_human if player_human.eq("PlayerM1") => obj.model_type = ModelType::PlayerHuman,

        player_zombie if player_zombie.eq("ZombiePlayer") => {
            obj.model_type = ModelType::PlayerHunter
        }

        zombie_normal if zombie_normal.contains("Bite") || zombie_normal.contains("Vira") => {
            obj.model_type = ModelType::ZombieNormal;
        }

        // NighyWalker Screamer GasTank Demoleasher Goon Toad_Stationary Bomber
        zombie_special
            if zombie_special.contains("NightW")
                || zombie_special.contains("Scre")
                || zombie_special.contains("GasT")
                || zombie_special.contains("Demol")
                || zombie_special.contains("Goo")
                || zombie_special.contains("Toad")
                || zombie_special.contains("Bomb") =>
        {
            obj.model_type = ModelType::ZombieSpecial;
        }

        // Volatile Volatile_Super
        zombie_hunter if zombie_hunter.contains("Vola") => {
            obj.model_type = ModelType::ZombieHunter;
        }

        // 赖斯士兵 enc_oldtown_bandit  0T_AirDrops_RaisBanditsRifle_Vest
        bandit if bandit.contains("enc") || bandit.contains("0T") => {
            obj.model_type = ModelType::SurvivorSpecial;
        }

        // Shopkeeper Spike
        survivor_shopkeeper
            if survivor_shopkeeper.contains("Sho") || survivor_shopkeeper.contains("Spik") =>
        {
            obj.model_type = ModelType::SurvivorShopkeeper;
        }

        _ => {
            if logo[1] == 0x20 {
                obj.model_type = ModelType::SurvivorNormal;
            }
        }
    }

    Some(obj)
}
