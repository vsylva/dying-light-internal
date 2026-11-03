#![allow(unsafe_op_in_unsafe_fn)]
#![allow(static_mut_refs)]
#![allow(deprecated)]

mod impls;

use std::{
    f32::consts::PI,
    ptr::{null, null_mut},
    sync::atomic::Ordering,
    thread::spawn,
};

use impls::{
    get_bone_joint_pos, get_screen_height, get_screen_width, is_in_frustum, point_to_screen, raytest_to_target,
};
use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::ScreenToClient,
    System::Memory::IsBadReadPtr,
    UI::{
        Input::KeyboardAndMouse::GetAsyncKeyState,
        WindowsAndMessaging::{FindWindowA, GetCursorPos},
    },
};

use crate::impls::{get_distance_to, get_position};

static mut ENGINE_DLL_INFO: libmem::Module =
    libmem::Module { base: 0, end: 0, size: 0, path: String::new(), name: String::new(), };

static mut GAME_DLL_INFO: libmem::Module =
    libmem::Module { base: 0, end: 0, size: 0, path: String::new(), name: String::new(), };

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
    &[EBones::Head, EBones::Neck, EBones::Spine3, EBones::Spine2, EBones::Spine1, EBones::Pelvis,],
    &[EBones::Neck, EBones::LUpperarm, EBones::LForearm, EBones::LHand,],
    &[EBones::Neck, EBones::RUpperarm, EBones::RForearm, EBones::RHand,],
    &[EBones::Pelvis, EBones::LThigh, EBones::LCalf, EBones::LFoot,],
    &[EBones::Pelvis, EBones::RThigh, EBones::RCalf, EBones::RFoot,],
];

const NOP_8: [u8; 8] = [0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,];
const PITCH_ORIGINAL: [u8; 8] = [0xF3, 0x0F, 0x11, 0x83, 0x78, 0x11, 0x00, 0x00,];
const YAW_ORIGINAL: [u8; 8] = [0xF3, 0x0F, 0x11, 0xB3, 0x74, 0x11, 0x00, 0x00,];

trait Ptr {
    unsafe fn is_bad_read_ptr(&self, size: usize,) -> bool;
}
impl<T,> Ptr for *const T {
    unsafe fn is_bad_read_ptr(&self, size: usize,) -> bool {
        IsBadReadPtr(Some(self.cast(),), size,).as_bool()
    }
}
impl<T,> Ptr for *mut T {
    unsafe fn is_bad_read_ptr(&self, size: usize,) -> bool {
        IsBadReadPtr(Some(self.cast(),), size,).as_bool()
    }
}

#[repr(C)]
#[derive(Default, PartialEq, Debug, Clone, Copy,)]
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result {
        match self
        {
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
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd,)]
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result {
        match self
        {
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

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd,)]
#[repr(i32)]
enum AimKeys {
    #[default]
    RMouseButton = 0x2,
    LCtrl = 0xA2,
    LShift = 0xA0,
    LAlt = 0xA4,
}

impl std::fmt::Display for AimKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result {
        match self
        {
            AimKeys::LShift => write!(f, "左Shift"),
            AimKeys::LAlt => write!(f, "左Alt"),
            AimKeys::RMouseButton => write!(f, "鼠标右"),
            AimKeys::LCtrl => write!(f, "左Ctrl"),
        }
    }
}

#[derive(Debug, Default,)]
struct World {
    game_p:                   *const CGame,
    game_di_p:                *const GameDI,
    session_cooperative_di_p: *const SessionCooperativeDI,
    level_di_p:               *const LevelDI,
    c_level_p:                *const CLevel,
    local_client_di_p:        *const LocalClientDI,
    player_di_p:              *mut PlayerDI,
    player_c_model_obj_p:     *const CModelObject,
    camera_manage_di_p:       *const CameraManagerDI,
    camera_fpp_di_p:          *const CameraFPPDI,
    player_world_pos_p:       *mut Vec3<f32,>,
    camera_angle_p:           *mut Vec2<f32,>,
}

static mut WORLD_MODEL_OBJ_ARRAY: Array<*const ModelObject,> = Array { ptr: null(), len: 0, max: 0, };

#[derive(Debug, Default, Clone,)]
struct Obj {
    model_obj_p:           *const ModelObject,
    c_model_obj_p:         *const CModelObject,
    c_model_obj_logo_p:    *const u32,
    c_model_obj_world_pos: Vec3<f32,>,
    model_obj_health_p:    *mut f32,
    model_obj_str_p:       *const i8,
    model_obj_str:         String,
    model_obj_type:        ModelType,
}

#[derive(Debug, Default,)]
#[repr(C)]
struct Array<T,> {
    ptr: *const T,
    len: u32,
    max: u32,
}

unsafe impl Send for Array<*const ModelObject,> {}
unsafe impl Sync for Array<*const ModelObject,> {}

#[derive(Clone, Copy, Default,)]
#[repr(C)]
struct Vec2<T,> {
    x: T,
    y: T,
}

#[derive(Clone, Copy, Default, Debug,)]
#[repr(C)]
struct Vec3<T,> {
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

#[derive(Debug, Clone,)]
struct Game {
    game_window: HWND,

    is_menu_on: bool,

    toggle_filter_zombie_normal:       bool,
    toggle_filter_zombie_special:      bool,
    toggle_filter_zombie_hunter:       bool,
    toggle_filter_survivor_normal:     bool,
    toggle_filter_survivor_special:    bool,
    toggle_filter_survivor_shopkeeper: bool,
    toggle_filter_player_human:        bool,
    toggle_filter_player_hunter:       bool,
    toggle_filter_other:               bool,

    toggle_draw_model_type_name:   bool,
    toggle_draw_bones:             bool,
    toggle_draw_visible_line:      bool,
    toggle_draw_type_data:         bool,
    toggle_draw_logo:              bool,
    toggle_draw_model_obj_p:       bool,
    toggle_draw_model_obj_p_array: bool,
    toggle_draw_world_data:        bool,

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

    color_zombie_normal:       [f32; 4],
    color_zombie_special:      [f32; 4],
    color_zombie_hunter:       [f32; 4],
    color_survivor_nomal:      [f32; 4],
    color_survivor_special:    [f32; 4],
    color_survivor_shopkeeper: [f32; 4],
    color_player_human:        [f32; 4],
    color_player_hunter:       [f32; 4],
    color_other:               [f32; 4],
}

impl Default for Game {
    fn default() -> Self {
        Self {
            aim_vk_code: AimKeys::RMouseButton as i32,

            aim_is_key_down: false,

            aim_is_mouse_patched: false,

            aim_locking_model_obj_p: null(),

            aim_best_closest_distance:    f32::MAX,
            aim_best_closest_model_obj_p: null(),

            aim_mouse_yaw_p:   0,
            aim_mouse_pitch_p: 0,

            game_window: HWND(null_mut(),),
            is_menu_on: true,

            aim_selected_bone: EBones::Head,

            toggle_filter_zombie_normal:       true,
            toggle_filter_zombie_special:      false,
            toggle_filter_zombie_hunter:       false,
            toggle_filter_survivor_normal:     false,
            toggle_filter_survivor_special:    false,
            toggle_filter_survivor_shopkeeper: false,
            toggle_filter_player_human:        false,
            toggle_filter_player_hunter:       false,
            toggle_filter_other:               false,

            toggle_draw_bones:             true,
            toggle_draw_visible_line:      false,
            toggle_draw_type_data:         false,
            toggle_draw_model_type_name:   true,
            toggle_draw_logo:              false,
            toggle_draw_model_obj_p:       false,
            toggle_draw_model_obj_p_array: false,
            toggle_draw_world_data:        false,

            aim_toggle:          true,
            aim_toggle_draw_fov: true,
            aim_fov:             150.0,
            aim_key_list:        [AimKeys::RMouseButton, AimKeys::LCtrl, AimKeys::LShift, AimKeys::LAlt,],
            aim_selected_key:    AimKeys::RMouseButton,

            color_zombie_normal:  [1.0, 1.0, 1.0, 1.0,],       // 白色
            color_zombie_special: [0.7569, 1.0, 0.7569, 1.0,], // 深海绿
            color_zombie_hunter:  [1.0, 0.0, 1.0, 1.0,],       // 紫红色

            color_survivor_nomal:      [0.2549, 0.4118, 0.8824, 1.0,], // 皇家蓝
            color_survivor_special:    [1.0, 1.0, 0.0, 1.0,],          // 黄色
            color_survivor_shopkeeper: [0.0, 1.0, 1.0, 1.0,],          // 青色

            color_player_human:  [0.0, 1.0, 0.0, 1.0,], // 绿色
            color_player_hunter: [1.0, 0.0, 0.0, 1.0,], // 红色
            color_other:         [1.0, 0.0, 0.0, 1.0,], // 白色
        }
    }
}

unsafe impl Send for Game {}

unsafe impl Sync for Game {}

use egui;

fn col(c: [f32; 4],) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (c[0].clamp(0.0, 1.0,) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0,) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0,) * 255.0) as u8,
        (c[3].clamp(0.0, 1.0,) * 255.0) as u8,
    )
}

fn color_edit(ui: &mut egui::Ui, id: &str, c: &mut [f32; 4],) {
    let mut rgb = [c[0], c[1], c[2], c[3],];
    let before = rgb;
    ui.color_edit_button_rgba_unmultiplied(&mut rgb,);
    let _ = id;
    if rgb != before
    {
        *c = rgb;
    }
}

unsafe fn draw_text(p: &egui::Painter, x: f32, y: f32, c: [f32; 4], s: &str,) {
    p.text(egui::pos2(x, y,), egui::Align2::LEFT_TOP, s, egui::FontId::proportional(20.0,), col(c,),);
}

unsafe fn draw_line(p: &egui::Painter, a: (f32, f32,), b: (f32, f32,), c: [f32; 4], w: f32,) {
    p.line_segment([egui::pos2(a.0, a.1,), egui::pos2(b.0, b.1,),], egui::Stroke::new(w, col(c,),),);
}

unsafe fn draw_circle(p: &egui::Painter, center: (f32, f32,), r: f32, c: [f32; 4],) {
    p.circle_stroke(egui::pos2(center.0, center.1,), r, egui::Stroke::new(1.5, col(c,),),);
}

static MENU_KEY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false,);

static GAME: std::sync::Mutex<Option<Game,>,> = std::sync::Mutex::new(None,);

struct Shell;

impl nany::Provider for Shell {
    unsafe fn init(&self, ctx: &egui::Context,) {
        nany::ui::install_font(ctx, "C:\\Windows\\Fonts\\simhei.ttf",);
        apply_theme(ctx,);
    }

    unsafe fn ui(&self, f: &nany::Frame<'_,>,) -> bool {
        f.ctx.request_repaint_after(std::time::Duration::from_millis(16,),);

        let mut guard = match GAME.lock()
        {
            Ok(g,) => g,
            Err(p,) => p.into_inner(),
        };
        let me: *mut Game = match guard.as_mut()
        {
            Some(g,) => g as *mut Game,
            None => return false,
        };

        // original approach: window messages do not work in this game, poll instead
        let mut mouse_pos = POINT::default();
        if GetCursorPos(&mut mouse_pos,).is_ok()
        {
            let mut p = mouse_pos;
            let _ = ScreenToClient((*me).game_window, &mut p,);
            nany::input::on_mouse_move(p.x, p.y,);
        }

        let left = GetAsyncKeyState(0x01,) as u16 & 0x8000 != 0;
        nany::input::on_mouse_button(0, left,);

        // original approach: local latch for the edge, framework key state may be reset
        let menu_key = GetAsyncKeyState(0x2D,) as u16 & 0x8000 != 0;
        if menu_key && !MENU_KEY.swap(true, Ordering::Relaxed,)
        {
            (*me).is_menu_on = !(*me).is_menu_on;
        }
        else if !menu_key
        {
            MENU_KEY.store(false, Ordering::Relaxed,);
        }

        let screen = (f.screen.0 as f32, f.screen.1 as f32,);
        let painter = f.ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("dl1_esp",),),);
        on_frame_draw(&mut *me, &painter, screen,);

        if !(*me).is_menu_on
        {
            return false;
        }

        draw_menu(&mut *me, f.ctx,);
        if let Some((x, y,),) = nany::input::pointer()
        {
            draw_cursor(f.ctx, x as f32, y as f32,);
        }
        true
    }
}

mod theme {
    use egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(11, 14, 20,);
    pub const CARD: Color32 = Color32::from_rgb(19, 24, 34,);
    pub const CARD_HI: Color32 = Color32::from_rgb(24, 30, 42,);
    pub const LINE: Color32 = Color32::from_rgb(32, 40, 54,);
    pub const TEXT: Color32 = Color32::from_rgb(230, 236, 245,);
    pub const MUTED: Color32 = Color32::from_rgb(124, 135, 152,);
    pub const ACCENT: Color32 = Color32::from_rgb(91, 140, 255,);
    pub const INPUT_BG: Color32 = Color32::from_rgb(15, 19, 27,);
}

fn apply_theme(ctx: &egui::Context) {
    ctx.all_styles_mut(|s| {
        s.visuals = egui::Visuals::dark();
        s.visuals.panel_fill = theme::BG;
        s.visuals.window_fill = theme::BG;
        s.visuals.extreme_bg_color = theme::INPUT_BG;
        s.visuals.faint_bg_color = theme::CARD;
        s.visuals.override_text_color = Some(theme::TEXT);
        s.visuals.widgets.inactive.bg_fill = theme::LINE;
        s.visuals.widgets.inactive.fg_stroke.color = theme::TEXT;
        s.visuals.widgets.hovered.bg_fill = theme::CARD_HI;
        s.visuals.widgets.hovered.fg_stroke.color = theme::TEXT;
        s.visuals.widgets.active.bg_fill = theme::ACCENT;
        s.visuals.widgets.active.fg_stroke.color = theme::TEXT;
        s.visuals.selection.bg_fill = theme::ACCENT;
        s.visuals.selection.stroke = egui::Stroke::new(1.5, theme::TEXT);
        s.visuals.window_stroke = egui::Stroke::new(1.0, theme::LINE);
        s.spacing.slider_width = 220.0;
        s.spacing.interact_size.y = 26.0;
        s.spacing.item_spacing = egui::vec2(8.0, 7.0);
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(17.0));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
        s.text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(14.0));
        s.text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(24.0));
        s.text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(16.0));
    });
}

unsafe fn draw_menu(game: &mut Game, ctx: &egui::Context) {
    egui::Window::new("dl1")
        .title_bar(false)
        .default_pos([100.0, 80.0])
        .default_size([860.0, 650.0])
        .resizable(false)
        .frame(egui::Frame::NONE.fill(theme::BG).stroke(egui::Stroke::new(1.0, theme::LINE)))
        .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(
                    egui::RichText::new("DL1")
                        .color(theme::TEXT)
                        .size(26.0)
                        .strong(),
                );
                ui.add_space(10.0);
                ui.label(egui::RichText::new("训练器").color(theme::MUTED).size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(14.0);
                    ui.label(egui::RichText::new("Insert 按键打开/关闭菜单").color(theme::MUTED).size(14.0));
                });
            });
            ui.add_space(8.0);
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.columns(2, |cols| {
                    card(&mut cols[0], "目标类型  ·  勾选即透视 + 可自瞄", |ui| {
                        type_row(
                            ui,
                            "丧尸",
                            &mut game.toggle_filter_zombie_normal,
                            &mut game.color_zombie_normal,
                        );
                        type_row(
                            ui,
                            "特感",
                            &mut game.toggle_filter_zombie_special,
                            &mut game.color_zombie_special,
                        );
                        type_row(
                            ui,
                            "夜魔",
                            &mut game.toggle_filter_zombie_hunter,
                            &mut game.color_zombie_hunter,
                        );
                        type_row(
                            ui,
                            "NPC",
                            &mut game.toggle_filter_survivor_normal,
                            &mut game.color_survivor_nomal,
                        );
                        type_row(
                            ui,
                            "强殖",
                            &mut game.toggle_filter_survivor_special,
                            &mut game.color_survivor_special,
                        );
                        type_row(
                            ui,
                            "商贩",
                            &mut game.toggle_filter_survivor_shopkeeper,
                            &mut game.color_survivor_shopkeeper,
                        );
                        type_row(
                            ui,
                            "人类",
                            &mut game.toggle_filter_player_human,
                            &mut game.color_player_human,
                        );
                        type_row(
                            ui,
                            "猎手",
                            &mut game.toggle_filter_player_hunter,
                            &mut game.color_player_hunter,
                        );
                        type_row(
                            ui,
                            "其他",
                            &mut game.toggle_filter_other,
                            &mut game.color_other,
                        );
                    });

                    card(&mut cols[1], "透视", |ui| {
                        ui.checkbox(&mut game.toggle_draw_model_type_name, "类型 + 距离");
                        ui.checkbox(&mut game.toggle_draw_bones, "骨骼");
                        ui.checkbox(&mut game.toggle_draw_visible_line, "可视线");
                        ui.checkbox(&mut game.toggle_draw_type_data, "模型名");
                        ui.checkbox(&mut game.toggle_draw_logo, "特征标志");
                        ui.checkbox(&mut game.toggle_draw_model_obj_p, "对象地址");
                        ui.checkbox(&mut game.toggle_draw_model_obj_p_array, "对象地址数组");
                        ui.checkbox(&mut game.toggle_draw_world_data, "世界地址");

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("自瞄  ·  跟随左侧勾选的类型")
                                .color(theme::MUTED)
                                .size(15.0),
                        );
                        ui.checkbox(&mut game.aim_toggle, "开启自瞄");
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("按键").color(theme::MUTED).size(15.0));
                            egui::ComboBox::from_id_salt("aim_key")
                                .width(110.0)
                                .selected_text(game.aim_selected_key.to_string())
                                .show_ui(ui, |ui| {
                                    for current in game.aim_key_list {
                                        if ui
                                            .selectable_label(
                                                game.aim_selected_key == current,
                                                current.to_string(),
                                            )
                                            .clicked()
                                        {
                                            game.aim_selected_key = current;
                                            game.aim_vk_code = current as i32;
                                        }
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("部位").color(theme::MUTED).size(15.0));
                            egui::ComboBox::from_id_salt("aim_bone")
                                .width(110.0)
                                .selected_text(game.aim_selected_bone.to_string())
                                .show_ui(ui, |ui| {
                                    for current in BONE_LIST {
                                        if ui
                                            .selectable_label(
                                                game.aim_selected_bone == current,
                                                current.to_string(),
                                            )
                                            .clicked()
                                        {
                                            game.aim_selected_bone = current;
                                        }
                                    }
                                });
                        });
                        ui.checkbox(&mut game.aim_toggle_draw_fov, "显示 FOV 圈");
                        ui.add(egui::Slider::new(&mut game.aim_fov, 50.0..=500.0).text("半径"));
                    });
                });
                ui.add_space(8.0);
            });
        });
}

fn card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(6.0);
    egui::Frame::NONE
        .fill(theme::CARD)
        .inner_margin(12)
        .corner_radius(8)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).color(theme::MUTED).size(14.0));
            ui.add_space(6.0);
            body(ui);
        });
    ui.add_space(2.0);
}

fn type_row(ui: &mut egui::Ui, label: &str, on: &mut bool, color: &mut [f32; 4]) {
    ui.horizontal(|ui| {
        ui.checkbox(on, egui::RichText::new(label).color(theme::TEXT).size(16.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            color_edit(ui, label, color);
        });
    });
}

unsafe fn on_frame_draw(game: &mut Game, p: &egui::Painter, screen: (f32, f32,),) {
    let world = match get_world()
    {
        Some(val,) => val,
        None => return,
    };

    if game.toggle_draw_model_obj_p_array
    {
        draw_text(
            p,
            0.0,
            0.0,
            game.color_zombie_normal,
            &format!(
                "ptr:{:p}\nlen:{}\nmax:{}",
                WORLD_MODEL_OBJ_ARRAY.ptr, WORLD_MODEL_OBJ_ARRAY.len, WORLD_MODEL_OBJ_ARRAY.max
            ),
        );
    }

    if game.toggle_draw_world_data
    {
        draw_text(p, 0.0, 60.0, game.color_zombie_normal, &format!("{:#?}", world),);
    }

    if game.aim_toggle_draw_fov
    {
        draw_circle(p, (screen.0 * 0.5, screen.1 * 0.5,), game.aim_fov, game.color_zombie_normal,);
    }

    for index in 0..WORLD_MODEL_OBJ_ARRAY.len
    {
        let model_obj_pp = WORLD_MODEL_OBJ_ARRAY.ptr.add(index as usize,);
        if model_obj_pp.is_bad_read_ptr(8,)
        {
            continue;
        }

        let mut model_obj_p = model_obj_pp.read();
        if model_obj_p.is_bad_read_ptr(8,)
        {
            continue;
        }

        model_obj_p = model_obj_p.byte_sub(0x18,);
        if model_obj_p.is_bad_read_ptr(8,)
        {
            continue;
        }

        let obj = match get_obj(model_obj_p,)
        {
            Some(val,) => val,
            None => continue,
        };

        if obj.c_model_obj_p == world.player_c_model_obj_p
        {
            continue;
        }

        if is_in_frustum(obj.model_obj_p,) == 0
        {
            continue;
        }

        let (filter, color,) = match obj.model_obj_type
        {
            ModelType::ZombieNormal => (game.toggle_filter_zombie_normal, game.color_zombie_normal,),
            ModelType::ZombieSpecial => (game.toggle_filter_zombie_special, game.color_zombie_special,),
            ModelType::ZombieHunter => (game.toggle_filter_zombie_hunter, game.color_zombie_hunter,),
            ModelType::SurvivorNormal => (game.toggle_filter_survivor_normal, game.color_survivor_nomal,),
            ModelType::SurvivorSpecial => (game.toggle_filter_survivor_special, game.color_survivor_special,),
            ModelType::SurvivorShopkeeper => (game.toggle_filter_survivor_shopkeeper, game.color_survivor_shopkeeper,),
            ModelType::PlayerHuman => (game.toggle_filter_player_human, game.color_player_human,),
            ModelType::PlayerHunter => (game.toggle_filter_player_hunter, game.color_player_hunter,),
            ModelType::Other => (game.toggle_filter_other, game.color_other,),
        };

        if !filter
        {
            continue;
        }

        let mut screen_pos: Vec2<f32,> = Vec2::default();
        point_to_screen(world.camera_fpp_di_p, &mut screen_pos, &obj.c_model_obj_world_pos,);

        if game.aim_toggle
        {
            aim_update_obj(game, &world, &obj,);
        }

        if game.toggle_draw_model_type_name
        {
            let data =
                format!("{}  {:.2}", obj.model_obj_type, get_distance_to(obj.model_obj_p, world.player_world_pos_p),);
            draw_text(p, screen_pos.x, screen_pos.y, color, &data,);
        }

        if game.toggle_draw_bones
        {
            draw_bones(p, &world, &obj, color,);
        }

        if game.toggle_draw_visible_line
        {
            let bone_world_pos: Vec3<f32,> = Vec3::default();
            get_bone_joint_pos(obj.model_obj_p, &bone_world_pos, game.aim_selected_bone as u8,);

            if raytest_to_target(
                obj.model_obj_p,
                obj.model_obj_p,
                get_position(world.camera_fpp_di_p,),
                &bone_world_pos,
                4,
            ) != 0
            {
                let mut bone_screen_pos: Vec2<f32,> = Vec2::default();
                point_to_screen(world.camera_fpp_di_p, &mut bone_screen_pos, &bone_world_pos,);
                draw_line(p, (screen.0 * 0.5, screen.1,), (bone_screen_pos.x, bone_screen_pos.y,), color, 2.0,);
            }
        }

        if game.toggle_draw_type_data
        {
            draw_text(p, screen_pos.x, screen_pos.y, color, obj.model_obj_str.as_str(),);
        }

        if game.toggle_draw_logo
        {
            let logo = format!("{:#X?}", obj.c_model_obj_logo_p.read());
            draw_text(p, screen_pos.x, screen_pos.y, color, &logo,);
        }

        if game.toggle_draw_model_obj_p
        {
            let ptrs = format!("model_obj_p: {:p}\nc_model_obj_p: {:p}", obj.model_obj_p, obj.c_model_obj_p);
            draw_text(p, screen_pos.x, screen_pos.y, color, &ptrs,);
        }
    }

    if game.aim_toggle
    {
        aim_lock_obj(game, &world, game.aim_selected_bone as u8,);
    }
}

unsafe fn draw_bones(p: &egui::Painter, world: &World, obj: &Obj, color: [f32; 4],) {
    let mut previous_world_pos: Vec3<f32,> = Vec3::default();
    let mut current_world_pos: Vec3<f32,> = Vec3::default();

    for bone_list in BONE_LISTS
    {
        previous_world_pos.x = 0.0;
        previous_world_pos.y = 0.0;
        previous_world_pos.z = 0.0;

        for bone in *bone_list
        {
            get_bone_joint_pos(obj.model_obj_p, &mut current_world_pos, *bone as u8,);

            if previous_world_pos.x == 0.0 && previous_world_pos.y == 0.0 && previous_world_pos.z == 0.0
            {
                previous_world_pos = current_world_pos;
                continue;
            }

            let mut previous_screen_pos: Vec2<f32,> = Vec2::default();
            let mut current_screen_pos: Vec2<f32,> = Vec2::default();

            point_to_screen(world.camera_fpp_di_p, &mut previous_screen_pos, &mut previous_world_pos,);
            point_to_screen(world.camera_fpp_di_p, &mut current_screen_pos, &mut current_world_pos,);

            draw_line(
                p,
                (previous_screen_pos.x, previous_screen_pos.y,),
                (current_screen_pos.x, current_screen_pos.y,),
                color,
                1.5,
            );

            previous_world_pos = current_world_pos;
        }
    }
}

#[inline(always)]
unsafe fn aim_update_obj(game: &mut Game, world: &World, obj: &Obj,) {
    if !match obj.model_obj_type
    {
        ModelType::Other | ModelType::SurvivorNormal | ModelType::SurvivorShopkeeper => return,
        ModelType::ZombieNormal => game.toggle_filter_zombie_normal,
        ModelType::ZombieSpecial => game.toggle_filter_zombie_special,
        ModelType::ZombieHunter => game.toggle_filter_zombie_hunter,
        ModelType::SurvivorSpecial => game.toggle_filter_survivor_special,
        ModelType::PlayerHuman => game.toggle_filter_player_human,
        ModelType::PlayerHunter => game.toggle_filter_player_hunter,
    }
    {
        return;
    }

    let mut screen_pos: Vec2<f32,> = Vec2::default();

    let world_pos: Vec3<f32,> = Vec3::default();

    let bone_p = get_bone_joint_pos(obj.model_obj_p, &world_pos, game.aim_selected_bone as u8,);
    if bone_p.is_null() || (world_pos.x == 0.0 && world_pos.y == 0.0 && world_pos.z == 0.0)
    {
        point_to_screen(world.camera_fpp_di_p, &mut screen_pos, &obj.c_model_obj_world_pos,);
    }
    else
    {
        point_to_screen(world.camera_fpp_di_p, &mut screen_pos, &world_pos,);
    }

    let pow_x = (screen_pos.x - get_screen_width(world.game_di_p,) as f32 / 2.0).powf(2.0,);

    let pow_y = (screen_pos.y - get_screen_height(world.game_di_p,) as f32 / 2.0).powf(2.0,);

    let distance = (pow_x + pow_y).sqrt();

    if distance > game.aim_fov || distance > game.aim_best_closest_distance
    {
        return;
    }

    game.aim_best_closest_distance = distance;
    game.aim_best_closest_model_obj_p = obj.model_obj_p;
}

#[inline(always)]
unsafe fn aim_lock_obj(game: &mut Game, world: &World, selected_bone: u8,) {
    if !game.aim_is_key_down
    {
        if game.aim_is_mouse_patched
        {
            game.aim_is_mouse_patched = false;
            mouse_unpatch(game,);
        }

        if game.aim_best_closest_distance == f32::MAX
        {
            return;
        }
    }

    if GetAsyncKeyState(game.aim_vk_code,) & 0x8000u16 as i16 != 0
    {
        if !game.aim_is_key_down
        {
            game.aim_is_key_down = true;

            game.aim_locking_model_obj_p = game.aim_best_closest_model_obj_p;

            if !game.aim_is_mouse_patched
            {
                game.aim_is_mouse_patched = true;
                mouse_patch(game,);
            }
        }

        if game.aim_locking_model_obj_p.is_bad_read_ptr(8,)
        {
            game.aim_is_key_down = false;

            if game.aim_is_mouse_patched
            {
                game.aim_is_mouse_patched = false;
                mouse_unpatch(game,);
            }

            return;
        }

        if game.aim_locking_model_obj_p.is_bad_read_ptr(8,)
        {
            game.aim_is_key_down = false;

            if game.aim_is_mouse_patched
            {
                game.aim_is_mouse_patched = false;
                mouse_unpatch(game,);
            }

            return;
        }

        if let None = get_obj(game.aim_locking_model_obj_p,)
        {
            game.aim_is_key_down = false;

            if game.aim_is_mouse_patched
            {
                game.aim_is_mouse_patched = false;
                mouse_unpatch(game,);
            }

            return;
        }

        let mut world_pos: Vec3<f32,> = Vec3::default();

        let bone_p = get_bone_joint_pos(game.aim_locking_model_obj_p, &mut world_pos, selected_bone,);
        if bone_p.is_null() || (world_pos.x == 0.0 && world_pos.y == 0.0 && world_pos.z == 0.0)
        {
            match get_obj(game.aim_locking_model_obj_p,)
            {
                Some(o,) => world_pos = o.c_model_obj_world_pos,
                None => return,
            }
        }

        let mut pos: Vec3<f32,> = Vec3::default();

        let camera_world_pos_p = get_position(world.camera_fpp_di_p,);

        pos.x = world_pos.x - camera_world_pos_p.read().x;
        pos.y = world_pos.z - camera_world_pos_p.read().z;
        pos.z = world_pos.y - camera_world_pos_p.read().y;

        let mut yaw = (pos.y / pos.x).atan() * 180.0 / PI;
        let pitch = (pos.z / (pos.x.powi(2,) + pos.y.powi(2,)).sqrt()).atan() * 180.0 / PI;

        if yaw < 0.0 && pos.y > 0.0
        {
            yaw += 180.0;
        }

        if yaw > 0.0 && pos.y < 0.0
        {
            yaw -= 180.0;
        }

        (*world.camera_angle_p).x = yaw;
        (*world.camera_angle_p).y = pitch;
    }
    else
    {
        game.aim_is_key_down = false;

        if game.aim_is_mouse_patched
        {
            game.aim_is_mouse_patched = false;
            mouse_unpatch(game,);
        }
    }

    game.aim_best_closest_distance = f32::MAX;
}

#[inline(always)]
unsafe fn mouse_patch(game: &mut Game,) {
    libmem::memory::write_memory_ex(&libmem::process::get_process().unwrap(), game.aim_mouse_yaw_p, NOP_8.as_slice(),);

    libmem::memory::write_memory_ex(&libmem::process::get_process().unwrap(), game.aim_mouse_pitch_p, NOP_8.as_slice(),);
}

#[inline(always)]
unsafe fn mouse_unpatch(game: &mut Game,) {
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
unsafe fn get_world() -> Option<World,> {
    let mut world = World {
        game_p:                   null(),
        game_di_p:                null(),
        session_cooperative_di_p: null(),
        level_di_p:               null(),
        c_level_p:                null(),
        local_client_di_p:        null(),
        player_di_p:              null_mut(),
        player_c_model_obj_p:     null(),

        camera_manage_di_p: null(),
        camera_fpp_di_p:    null(),

        player_world_pos_p: null_mut(),
        camera_angle_p:     null_mut(),
    };

    // CGame
    world.game_p = CGAME_PP.read();

    // GameDI
    let game_di_pp = world.game_p.byte_add(0x98,).cast::<*const GameDI>();
    if game_di_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.game_di_p = game_di_pp.read();
    if world.game_di_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // SessionCooperativeDI
    let session_cooperative_di_pp = world.game_di_p.byte_add(0x540,).cast::<*const SessionCooperativeDI>();
    if session_cooperative_di_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.session_cooperative_di_p = session_cooperative_di_pp.read();
    if world.session_cooperative_di_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // LevelDI
    let level_di_pp = world.session_cooperative_di_p.byte_add(0xB0,).cast::<*const LevelDI>();
    if level_di_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.level_di_p = level_di_pp.read();
    if world.level_di_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // CLevel
    let c_level_pp = world.level_di_p.byte_add(0x8,).cast::<*const CLevel>();
    if c_level_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.c_level_p = c_level_pp.read();
    if world.c_level_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // LocalClientDI
    let local_client_pp = world.session_cooperative_di_p.byte_add(0xB8,).cast::<*const LocalClientDI>();
    if local_client_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.local_client_di_p = local_client_pp.read();
    if world.local_client_di_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // PlayerDI
    let player_di_pp = world.local_client_di_p.byte_add(0x50,).cast::<*mut PlayerDI>();
    if player_di_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.player_di_p = player_di_pp.read();
    if world.player_di_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // PlayerPos
    world.player_world_pos_p = world.player_di_p.byte_add(0x7B0,).cast::<Vec3<f32,>>();

    if world.player_world_pos_p.is_bad_read_ptr(12,)
    {
        return None;
    }
    if world.player_world_pos_p.read().y == 0.0
    {
        return None;
    }

    // Angle
    world.camera_angle_p = world.player_di_p.byte_add(0x111C,).cast::<Vec2<f32,>>();

    if world.camera_angle_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // PlayerCModelObject
    let player_c_model_obj_pp = world.player_di_p.byte_sub(0x50,).cast::<*const CModelObject>();
    if player_c_model_obj_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.player_c_model_obj_p = player_c_model_obj_pp.read();
    if world.player_c_model_obj_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // CameraManagerDI
    let camera_manager_pp = world.session_cooperative_di_p.byte_add(0xC0,).cast::<*const CameraManagerDI>();
    if camera_manager_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.camera_manage_di_p = camera_manager_pp.read();
    if world.camera_manage_di_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // CameraFPPDI
    let camera_fpp_di_pp = world.camera_manage_di_p.byte_add(0x50,).cast::<*const CameraFPPDI>();
    if camera_fpp_di_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    world.camera_fpp_di_p = camera_fpp_di_pp.read();
    if world.camera_fpp_di_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    impls::get_objects_in_frustum(world.camera_fpp_di_p, &raw const WORLD_MODEL_OBJ_ARRAY, 0.0,);

    Some(world,)
}

#[inline(always)]
unsafe fn get_obj(model_obj_p: *const ModelObject,) -> Option<Obj,> {
    let mut obj = Obj { model_obj_p, ..Default::default() };

    let c_model_obj_pp = obj.model_obj_p.byte_add(0x20,).cast::<*const CModelObject>();
    if c_model_obj_pp.is_bad_read_ptr(8,)
    {
        return None;
    }

    // CModelObject
    obj.c_model_obj_p = c_model_obj_pp.read();
    if obj.c_model_obj_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    // Logo
    obj.c_model_obj_logo_p = obj.c_model_obj_p.byte_add(0x340,).cast::<u32>();
    if obj.c_model_obj_logo_p.is_bad_read_ptr(4,)
    {
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

    match obj.c_model_obj_logo_p.read()
    {
        0x0 | 0x1 | 0x2 | 0x8 => return None,
        _ => (),
    }

    let world_pos_x = obj.c_model_obj_p.byte_add(0x11C,).cast::<f32>();
    let world_pos_y = obj.c_model_obj_p.byte_add(0x12C,).cast::<f32>();
    let world_pos_z = obj.c_model_obj_p.byte_add(0x13C,).cast::<f32>();

    if world_pos_x.is_bad_read_ptr(4,) || world_pos_y.is_bad_read_ptr(4,) || world_pos_z.is_bad_read_ptr(4,)
    {
        return None;
    }

    obj.c_model_obj_world_pos.x = world_pos_x.read();
    obj.c_model_obj_world_pos.y = world_pos_y.read();
    obj.c_model_obj_world_pos.z = world_pos_z.read();

    // get_world_position(obj.model_obj_p, &obj.c_model_obj_world_pos);

    if obj.c_model_obj_world_pos.x == 0.0 && obj.c_model_obj_world_pos.y == 0.0 && obj.c_model_obj_world_pos.z == 0.0
    {
        return None;
    }

    // ModelObjectHealth
    let health_module_pp = obj.model_obj_p.byte_add(0xCE8,).cast::<*const HealthModule>();
    if health_module_pp.is_bad_read_ptr(8,)
    {
        return None;
    }
    let model_health_p = health_module_pp.read();
    if model_health_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    obj.model_obj_health_p = model_health_p.byte_add(0x78,).cast::<f32>().cast_mut();
    if obj.model_obj_health_p.is_bad_read_ptr(4,)
    {
        return None;
    }
    if obj.model_obj_health_p.read() == 0.0
    {
        return None;
    }

    // ModelObjectTypeData
    let model_type_data_pp = obj.c_model_obj_p.byte_add(0x60,).cast::<*const i8>();
    if model_type_data_pp.is_bad_read_ptr(8,)
    {
        return None;
    }

    obj.model_obj_str_p = model_type_data_pp.read();
    if obj.model_obj_str_p.is_bad_read_ptr(8,)
    {
        return None;
    }

    obj.model_obj_str = std::ffi::CStr::from_ptr(obj.model_obj_str_p,).to_string_lossy().to_string();

    let bytes = obj.model_obj_str.as_bytes();

    let start = bytes.iter().position(|&b| b == b';',)? + 1;
    if start >= bytes.len()
    {
        return None;
    }

    match &bytes[start..]
    {
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
        b if b.starts_with(b"Bi",) || b.starts_with(b"Vi",) || b.starts_with(b"Dea",) =>
        {
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

        b if b.starts_with(b"Vo",) => obj.model_obj_type = ModelType::ZombieHunter,

        b if b.starts_with(b"Zo",) || b.starts_with(b"DW",) =>
        {
            // DW_Zombie
            obj.model_obj_type = ModelType::PlayerHunter
        }

        // enc很多是中立    Quest_GoodNight是友好NPC
        b if b.starts_with(b"en",) || b.starts_with(b"0T",) => obj.model_obj_type = ModelType::SurvivorSpecial,

        b if b.starts_with(b"Sh",) || b.starts_with(b"Sp",) => obj.model_obj_type = ModelType::SurvivorShopkeeper,

        b if b.starts_with(b"Pl",) => obj.model_obj_type = ModelType::PlayerHuman,

        // 塔楼上面的坐在桌子面前操作的机械工
        // 泽雷博士车门前躺着的马里克
        // b if b.starts_with(b"Hub") || b.starts_with(b"Maa") => obj.model_obj_type = ModelType::SurvivorNormal,
        _ =>
        {
            if obj.c_model_obj_logo_p.read() == 0x2000
            {
                obj.model_obj_type = ModelType::SurvivorNormal
            }
            else
            {
                obj.model_obj_type = ModelType::Other
            }
        }
    };

    Some(obj,)
}

// #[target_feature(enable = "sse")]
// unsafe  fn sse_get_distance_to(pos1: Vec3<f32>, pos2: Vec3<f32>) -> f32 {
//     let a = std::arch::x86_64::_mm_set_ps(0.0, pos1.z, pos1.y, pos1.x);
//     let b = std::arch::x86_64::_mm_set_ps(0.0, pos2.z, pos2.y, pos2.x);
//     let diff = std::arch::x86_64::_mm_sub_ps(b, a);
//     let sq = std::arch::x86_64::_mm_mul_ps(diff, diff);

//     let mut result = [0.0f32; 4];
//     std::arch::x86_64::_mm_storeu_ps(result.as_mut_ptr(), sq);
//     (result[0] + result[1] + result[2]) / 10.0
// }

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(h_module: isize, ul_reason_for_call: u32, _: *const core::ffi::c_void,) -> i32 {
    if ul_reason_for_call == 1
    {
        let _ = windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls(windows::Win32::Foundation::HMODULE(
            h_module as *mut core::ffi::c_void,
        ),);

        spawn(move || {
            loop
            {
                if let Some(engine_info,) = libmem::find_module("engine_x64_rwdi.dll",)
                    && let Some(gamedll_info,) = libmem::find_module("gamedll_x64_rwdi.dll",)
                    && engine_info.base != 0
                    && gamedll_info.base != 0
                {
                    ENGINE_DLL_INFO = engine_info;
                    GAME_DLL_INFO = gamedll_info;
                    break;
                }

                std::thread::sleep(std::time::Duration::from_secs(5,),);
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

            let c_game_pp_offset = c_game_pp.byte_add(3,).cast::<u32>().read_unaligned() + 7;

            CGAME_PP = c_game_pp.byte_add(c_game_pp_offset as usize,).cast();

            while let None = get_world()
            {
                std::thread::sleep(std::time::Duration::from_secs(5,),);
            }

            game.game_window = match FindWindowA(
                windows::core::PCSTR::from_raw(b"techland_game_class\0".as_ptr(),),
                windows::core::PCSTR::null(),
            )
            {
                Ok(h,) => h,
                Err(_,) => HWND(null_mut(),),
            };

            match GAME.lock()
            {
                Ok(mut g,) => *g = Some(game,),
                Err(p,) => *p.into_inner() = Some(game,),
            }
            nany::on_dll_attach_tagged("dl1", Box::new(Shell,),);
        },);
    }

    1
}

unsafe fn draw_cursor(ctx: &egui::Context, x: f32, y: f32,) {
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("dl1_cursor",),),);
    let pos = egui::pos2(x, y,);
    let body = [
        pos,
        pos + egui::vec2(0.0, 20.0,),
        pos + egui::vec2(5.2, 14.6,),
        pos + egui::vec2(12.0, 14.0,),
        pos + egui::vec2(13.6, 9.2,),
        pos + egui::vec2(6.8, 9.8,),
    ];
    p.add(egui::Shape::convex_polygon(
        body.to_vec(),
        egui::Color32::WHITE,
        egui::Stroke::new(1.2, egui::Color32::from_rgb(16, 18, 22,),),
    ),);
}
