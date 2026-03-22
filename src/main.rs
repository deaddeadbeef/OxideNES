#![windows_subsystem = "windows"]

use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{traits::*, HeapRb};
use gilrs::{Gilrs, Button, Axis};
use serde::{Serialize, Deserialize};

use oxidenes::bus::Bus;
use oxidenes::cartridge::Cartridge;
use oxidenes::cpu::Cpu;
use oxidenes::joypad::JoypadButton;
use oxidenes::netplay::{NetplaySession, NetplayState};
use oxidenes::ppu::Region;
use oxidenes::scripting::ScriptEngine;
use oxidenes::achievements::{AchievementEngine, md5_hex};
use oxidenes::recording::{InputRecording, sha256};
use oxidenes::romdb::RomDatabase;
use oxidenes::updater::Updater;

// Single source of truth for all screen/window dimensions
const TV_WIDTH: usize = 1200;
const TV_HEIGHT: usize = 900;
const CONSOLE_HEIGHT: usize = 160;
const WINDOW_WIDTH: usize = TV_WIDTH;
const WINDOW_HEIGHT: usize = TV_HEIGHT + CONSOLE_HEIGHT;
const SCREEN_W: usize = 820;
const SCREEN_H: usize = 769;
const SCREEN_X: usize = 190;
const SCREEN_Y: usize = 50;

// NES menu colors
const MENU_BG: u32 = 0x0C0C3C;
const MENU_WHITE: u32 = 0xFCFCFC;
const MENU_GOLD: u32 = 0xF8D878;
const MENU_GRAY: u32 = 0x9C9C9C;
const MENU_DARK_GRAY: u32 = 0x585858;
const MENU_LIGHT_BLUE: u32 = 0x6888FC;

// =====================================================================
// Config persistence
// =====================================================================

#[derive(Serialize, Deserialize, Clone)]
struct KeyBindings {
    up: String,
    down: String,
    left: String,
    right: String,
    a: String,
    b: String,
    start: String,
    select: String,
    turbo_a: String,
    turbo_b: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct KeyboardBindings {
    up: String,
    down: String,
    left: String,
    right: String,
    a: String,
    b: String,
    start: String,
    select: String,
    turbo_a: String,
    turbo_b: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ControllerBindings {
    a: String,
    b: String,
    turbo_a: String,
    turbo_b: String,
    start: String,
    select: String,
    deadzone: f32,
}

#[derive(Serialize, Deserialize, Clone)]
struct InputBindings {
    keyboard_p1: KeyboardBindings,
    keyboard_p2: KeyboardBindings,
    controller_p1: ControllerBindings,
    controller_p2: ControllerBindings,
}

#[derive(Default, Clone)]
struct StickState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl StickState {
    #[inline]
    fn any_active(&self) -> bool {
        self.up || self.down || self.left || self.right
    }
    fn clear(&mut self) {
        self.up = false;
        self.down = false;
        self.left = false;
        self.right = false;
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            up: "W".to_string(),
            down: "S".to_string(),
            left: "A".to_string(),
            right: "D".to_string(),
            a: "K".to_string(),
            b: "J".to_string(),
            start: "Enter".to_string(),
            select: "RightShift".to_string(),
            turbo_a: "Z".to_string(),
            turbo_b: "X".to_string(),
        }
    }
}

impl Default for KeyboardBindings {
    fn default() -> Self {
        Self {
            up: "W".to_string(),
            down: "S".to_string(),
            left: "A".to_string(),
            right: "D".to_string(),
            a: "K".to_string(),
            b: "J".to_string(),
            start: "Enter".to_string(),
            select: "RightShift".to_string(),
            turbo_a: "Z".to_string(),
            turbo_b: "X".to_string(),
        }
    }
}

impl Default for ControllerBindings {
    fn default() -> Self {
        Self {
            a: "South".to_string(),
            b: "West".to_string(),
            turbo_a: "East".to_string(),
            turbo_b: "North".to_string(),
            start: "Start".to_string(),
            select: "Select".to_string(),
            deadzone: 0.3,
        }
    }
}

impl Default for InputBindings {
    fn default() -> Self {
        Self {
            keyboard_p1: KeyboardBindings::default(),
            keyboard_p2: KeyboardBindings {
                up: "Up".to_string(),
                down: "Down".to_string(),
                left: "Left".to_string(),
                right: "Right".to_string(),
                a: "Period".to_string(),
                b: "Comma".to_string(),
                start: "Slash".to_string(),
                select: "RightCtrl".to_string(),
                turbo_a: "Semicolon".to_string(),
                turbo_b: "Apostrophe".to_string(),
            },
            controller_p1: ControllerBindings::default(),
            controller_p2: ControllerBindings::default(),
        }
    }
}

fn string_to_key(s: &str) -> Option<Key> {
    match s {
        "A" => Some(Key::A), "B" => Some(Key::B), "C" => Some(Key::C), "D" => Some(Key::D),
        "E" => Some(Key::E), "F" => Some(Key::F), "G" => Some(Key::G), "H" => Some(Key::H),
        "I" => Some(Key::I), "J" => Some(Key::J), "K" => Some(Key::K), "L" => Some(Key::L),
        "M" => Some(Key::M), "N" => Some(Key::N), "O" => Some(Key::O), "P" => Some(Key::P),
        "Q" => Some(Key::Q), "R" => Some(Key::R), "S" => Some(Key::S), "T" => Some(Key::T),
        "U" => Some(Key::U), "V" => Some(Key::V), "W" => Some(Key::W), "X" => Some(Key::X),
        "Y" => Some(Key::Y), "Z" => Some(Key::Z),
        "Up" => Some(Key::Up), "Down" => Some(Key::Down), "Left" => Some(Key::Left), "Right" => Some(Key::Right),
        "Enter" => Some(Key::Enter), "Space" => Some(Key::Space),
        "LeftShift" => Some(Key::LeftShift), "RightShift" => Some(Key::RightShift),
        "LeftCtrl" => Some(Key::LeftCtrl), "RightCtrl" => Some(Key::RightCtrl),
        "Comma" => Some(Key::Comma), "Period" => Some(Key::Period),
        "Slash" => Some(Key::Slash), "Semicolon" => Some(Key::Semicolon),
        "Apostrophe" => Some(Key::Apostrophe),
        "1" => Some(Key::Key1), "2" => Some(Key::Key2), "3" => Some(Key::Key3),
        "4" => Some(Key::Key4), "5" => Some(Key::Key5), "6" => Some(Key::Key6),
        "7" => Some(Key::Key7), "8" => Some(Key::Key8), "9" => Some(Key::Key9), "0" => Some(Key::Key0),
        "Escape" => Some(Key::Escape), "Tab" => Some(Key::Tab), "Backspace" => Some(Key::Backspace),
        "Delete" => Some(Key::Delete), "Insert" => Some(Key::Insert),
        "Home" => Some(Key::Home), "End" => Some(Key::End),
        "PageUp" => Some(Key::PageUp), "PageDown" => Some(Key::PageDown),
        "Pause" => Some(Key::Pause), "Menu" => Some(Key::Menu),
        "F1" => Some(Key::F1), "F2" => Some(Key::F2), "F3" => Some(Key::F3),
        "F4" => Some(Key::F4), "F5" => Some(Key::F5), "F6" => Some(Key::F6),
        "F7" => Some(Key::F7), "F8" => Some(Key::F8), "F9" => Some(Key::F9),
        "F10" => Some(Key::F10), "F11" => Some(Key::F11), "F12" => Some(Key::F12),
        "F13" => Some(Key::F13), "F14" => Some(Key::F14), "F15" => Some(Key::F15),
        "CapsLock" => Some(Key::CapsLock), "NumLock" => Some(Key::NumLock), "ScrollLock" => Some(Key::ScrollLock),
        "NumPad0" => Some(Key::NumPad0), "NumPad1" => Some(Key::NumPad1), "NumPad2" => Some(Key::NumPad2),
        "NumPad3" => Some(Key::NumPad3), "NumPad4" => Some(Key::NumPad4), "NumPad5" => Some(Key::NumPad5),
        "NumPad6" => Some(Key::NumPad6), "NumPad7" => Some(Key::NumPad7), "NumPad8" => Some(Key::NumPad8),
        "NumPad9" => Some(Key::NumPad9),
        "NumPadDot" => Some(Key::NumPadDot), "NumPadSlash" => Some(Key::NumPadSlash),
        "NumPadAsterisk" => Some(Key::NumPadAsterisk), "NumPadMinus" => Some(Key::NumPadMinus),
        "NumPadPlus" => Some(Key::NumPadPlus), "NumPadEnter" => Some(Key::NumPadEnter),
        "LeftAlt" => Some(Key::LeftAlt), "RightAlt" => Some(Key::RightAlt),
        "LeftSuper" => Some(Key::LeftSuper), "RightSuper" => Some(Key::RightSuper),
        "Backquote" => Some(Key::Backquote), "Backslash" => Some(Key::Backslash),
        "Equal" => Some(Key::Equal), "Minus" => Some(Key::Minus),
        "LeftBracket" => Some(Key::LeftBracket), "RightBracket" => Some(Key::RightBracket),
        _ => None,
    }
}

fn string_to_gilrs_button(name: &str) -> Option<gilrs::Button> {
    match name {
        "South" => Some(gilrs::Button::South),
        "East" => Some(gilrs::Button::East),
        "North" => Some(gilrs::Button::North),
        "West" => Some(gilrs::Button::West),
        "Start" => Some(gilrs::Button::Start),
        "Select" => Some(gilrs::Button::Select),
        "Mode" => Some(gilrs::Button::Mode),
        "LeftTrigger" => Some(gilrs::Button::LeftTrigger),
        "RightTrigger" => Some(gilrs::Button::RightTrigger),
        "LeftTrigger2" => Some(gilrs::Button::LeftTrigger2),
        "RightTrigger2" => Some(gilrs::Button::RightTrigger2),
        "LeftThumb" => Some(gilrs::Button::LeftThumb),
        "RightThumb" => Some(gilrs::Button::RightThumb),
        "DPadUp" => Some(gilrs::Button::DPadUp),
        "DPadDown" => Some(gilrs::Button::DPadDown),
        "DPadLeft" => Some(gilrs::Button::DPadLeft),
        "DPadRight" => Some(gilrs::Button::DPadRight),
        _ => None,
    }
}

fn key_to_string(key: Key) -> String {
    match key {
        Key::A => "A".to_string(), Key::B => "B".to_string(), Key::C => "C".to_string(), Key::D => "D".to_string(),
        Key::E => "E".to_string(), Key::F => "F".to_string(), Key::G => "G".to_string(), Key::H => "H".to_string(),
        Key::I => "I".to_string(), Key::J => "J".to_string(), Key::K => "K".to_string(), Key::L => "L".to_string(),
        Key::M => "M".to_string(), Key::N => "N".to_string(), Key::O => "O".to_string(), Key::P => "P".to_string(),
        Key::Q => "Q".to_string(), Key::R => "R".to_string(), Key::S => "S".to_string(), Key::T => "T".to_string(),
        Key::U => "U".to_string(), Key::V => "V".to_string(), Key::W => "W".to_string(), Key::X => "X".to_string(),
        Key::Y => "Y".to_string(), Key::Z => "Z".to_string(),
        Key::Up => "Up".to_string(), Key::Down => "Down".to_string(), Key::Left => "Left".to_string(), Key::Right => "Right".to_string(),
        Key::Enter => "Enter".to_string(), Key::Space => "Space".to_string(),
        Key::LeftShift => "LeftShift".to_string(), Key::RightShift => "RightShift".to_string(),
        Key::LeftCtrl => "LeftCtrl".to_string(), Key::RightCtrl => "RightCtrl".to_string(),
        Key::Comma => "Comma".to_string(), Key::Period => "Period".to_string(),
        Key::Slash => "Slash".to_string(), Key::Semicolon => "Semicolon".to_string(),
        Key::Apostrophe => "Apostrophe".to_string(),
        Key::Key1 => "1".to_string(), Key::Key2 => "2".to_string(), Key::Key3 => "3".to_string(),
        Key::Key4 => "4".to_string(), Key::Key5 => "5".to_string(), Key::Key6 => "6".to_string(),
        Key::Key7 => "7".to_string(), Key::Key8 => "8".to_string(), Key::Key9 => "9".to_string(), Key::Key0 => "0".to_string(),
        Key::Escape => "Escape".to_string(), Key::Tab => "Tab".to_string(), Key::Backspace => "Backspace".to_string(),
        Key::Delete => "Delete".to_string(), Key::Insert => "Insert".to_string(),
        Key::Home => "Home".to_string(), Key::End => "End".to_string(),
        Key::PageUp => "PageUp".to_string(), Key::PageDown => "PageDown".to_string(),
        Key::Pause => "Pause".to_string(), Key::Menu => "Menu".to_string(),
        Key::F1 => "F1".to_string(), Key::F2 => "F2".to_string(), Key::F3 => "F3".to_string(),
        Key::F4 => "F4".to_string(), Key::F5 => "F5".to_string(), Key::F6 => "F6".to_string(),
        Key::F7 => "F7".to_string(), Key::F8 => "F8".to_string(), Key::F9 => "F9".to_string(),
        Key::F10 => "F10".to_string(), Key::F11 => "F11".to_string(), Key::F12 => "F12".to_string(),
        Key::F13 => "F13".to_string(), Key::F14 => "F14".to_string(), Key::F15 => "F15".to_string(),
        Key::CapsLock => "CapsLock".to_string(), Key::NumLock => "NumLock".to_string(), Key::ScrollLock => "ScrollLock".to_string(),
        Key::NumPad0 => "NumPad0".to_string(), Key::NumPad1 => "NumPad1".to_string(), Key::NumPad2 => "NumPad2".to_string(),
        Key::NumPad3 => "NumPad3".to_string(), Key::NumPad4 => "NumPad4".to_string(), Key::NumPad5 => "NumPad5".to_string(),
        Key::NumPad6 => "NumPad6".to_string(), Key::NumPad7 => "NumPad7".to_string(), Key::NumPad8 => "NumPad8".to_string(),
        Key::NumPad9 => "NumPad9".to_string(),
        Key::NumPadDot => "NumPadDot".to_string(), Key::NumPadSlash => "NumPadSlash".to_string(),
        Key::NumPadAsterisk => "NumPadAsterisk".to_string(), Key::NumPadMinus => "NumPadMinus".to_string(),
        Key::NumPadPlus => "NumPadPlus".to_string(), Key::NumPadEnter => "NumPadEnter".to_string(),
        Key::LeftAlt => "LeftAlt".to_string(), Key::RightAlt => "RightAlt".to_string(),
        Key::LeftSuper => "LeftSuper".to_string(), Key::RightSuper => "RightSuper".to_string(),
        Key::Backquote => "Backquote".to_string(), Key::Backslash => "Backslash".to_string(),
        Key::Equal => "Equal".to_string(), Key::Minus => "Minus".to_string(),
        Key::LeftBracket => "LeftBracket".to_string(), Key::RightBracket => "RightBracket".to_string(),
        _ => format!("{:?}", key),
    }
}

fn gilrs_button_to_string(button: gilrs::Button) -> String {
    match button {
        gilrs::Button::South => "South".to_string(),
        gilrs::Button::East => "East".to_string(),
        gilrs::Button::North => "North".to_string(),
        gilrs::Button::West => "West".to_string(),
        gilrs::Button::Start => "Start".to_string(),
        gilrs::Button::Select => "Select".to_string(),
        gilrs::Button::Mode => "Mode".to_string(),
        gilrs::Button::LeftTrigger => "LeftTrigger".to_string(),
        gilrs::Button::RightTrigger => "RightTrigger".to_string(),
        gilrs::Button::LeftTrigger2 => "LeftTrigger2".to_string(),
        gilrs::Button::RightTrigger2 => "RightTrigger2".to_string(),
        gilrs::Button::LeftThumb => "LeftThumb".to_string(),
        gilrs::Button::RightThumb => "RightThumb".to_string(),
        gilrs::Button::DPadUp => "DPadUp".to_string(),
        gilrs::Button::DPadDown => "DPadDown".to_string(),
        gilrs::Button::DPadLeft => "DPadLeft".to_string(),
        gilrs::Button::DPadRight => "DPadRight".to_string(),
        _ => format!("{:?}", button),
    }
}

fn default_region() -> String { "ntsc".to_string() }
fn default_glass_intensity() -> u8 { 60 }
fn default_true() -> bool { true }

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum CrtMaskMode {
    Off,
    ShadowMask,
    ApertureGrille,
    SlotMask,
}

impl Default for CrtMaskMode {
    fn default() -> Self { CrtMaskMode::SlotMask }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct CrtConfig {
    scanline_intensity: u8,   // 0-100, default 15
    phosphor_warmth: u8,      // 0-100, default 20
    vignette_strength: u8,    // 0-100, default 20
    blur_amount: u8,          // 0-100, default 0
    curvature_strength: u8,   // 0-100, default 15
    mask_mode: CrtMaskMode,
    mask_intensity: u8,       // 0-100, how strongly the mask pattern shows
}

impl Default for CrtConfig {
    fn default() -> Self {
        Self {
            scanline_intensity: 40,
            phosphor_warmth: 30,
            vignette_strength: 20,
            blur_amount: 0,
            curvature_strength: 15,
            mask_mode: CrtMaskMode::SlotMask,
            mask_intensity: 50,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct EmulatorConfig {
    recent_games: Vec<String>,
    crt_enabled: bool,
    barrel_distortion: bool,
    audio_volume: u32,
    #[serde(default)]
    key_bindings: Option<KeyBindings>,
    #[serde(default = "default_region")]
    region: String,
    #[serde(default)]
    input_bindings: InputBindings,
    #[serde(default = "default_glass_intensity")]
    glass_intensity: u8,
    #[serde(default)]
    config_version: u32,
    #[serde(default)]
    crt_config: CrtConfig,
    #[serde(default = "default_true")]
    check_for_updates: bool,
    #[serde(default)]
    favorite_games: Vec<String>,
    #[serde(default)]
    rom_directory: Option<String>,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            recent_games: Vec::new(),
            crt_enabled: true,
            barrel_distortion: false,
            audio_volume: 100,
            key_bindings: None,
            region: "ntsc".to_string(),
            input_bindings: InputBindings::default(),
            glass_intensity: 60,
            config_version: 3,
            crt_config: CrtConfig::default(),
            check_for_updates: true,
            favorite_games: Vec::new(),
            rom_directory: None,
        }
    }
}

fn config_dir() -> PathBuf {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".nes-emulator")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn load_config() -> EmulatorConfig {
    let path = config_path();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(mut cfg) = serde_json::from_str::<EmulatorConfig>(&data) {
                let mut migrated = false;
                
                // Handle migration from old key_bindings to new input_bindings
                if cfg.config_version < 2 {
                    if let Some(old) = &cfg.key_bindings {
                        cfg.input_bindings.keyboard_p1 = KeyboardBindings {
                            up: old.up.clone(),
                            down: old.down.clone(),
                            left: old.left.clone(),
                            right: old.right.clone(),
                            a: old.a.clone(),
                            b: old.b.clone(),
                            start: old.start.clone(),
                            select: old.select.clone(),
                            turbo_a: old.turbo_a.clone(),
                            turbo_b: old.turbo_b.clone(),
                        };
                        cfg.key_bindings = None;
                    }
                    cfg.config_version = 2;
                    migrated = true;
                }
                
                if cfg.config_version < 3 {
                    // rom_directory defaults to None via serde(default) — triggers first-run setup
                    cfg.config_version = 3;
                    migrated = true;
                }
                
                if migrated {
                    save_config(&cfg);
                }
                return cfg;
            }
        }
    }
    let cfg = EmulatorConfig::default();
    save_config(&cfg);
    cfg
}

fn save_config(cfg: &EmulatorConfig) {
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(data) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(config_path(), data);
    }
}

fn add_recent_game(cfg: &mut EmulatorConfig, path: &str) {
    cfg.recent_games.retain(|p| p != path);
    cfg.recent_games.insert(0, path.to_string());
    cfg.recent_games.truncate(10);
}

fn toggle_favorite(config: &mut EmulatorConfig, path: &str) -> bool {
    if let Some(pos) = config.favorite_games.iter().position(|g| g == path) {
        config.favorite_games.remove(pos);
        false // removed
    } else {
        config.favorite_games.push(path.to_string());
        true // added
    }
}

fn is_favorite(config: &EmulatorConfig, path: &str) -> bool {
    config.favorite_games.iter().any(|g| g == path)
}

// =====================================================================
// Save state support (SRAM battery backup emulation)
// =====================================================================

fn save_state_dir() -> PathBuf {
    config_dir().join("saves")
}

// =====================================================================
// Game Genie cheat code persistence
// =====================================================================

fn cheats_dir() -> PathBuf {
    config_dir().join("cheats")
}

fn save_cheats(rom_name: &str, cheats: &[oxidenes::bus::GameGenieCode]) {
    if rom_name.is_empty() { return; }
    let dir = cheats_dir();
    let _ = fs::create_dir_all(&dir);
    let entries: Vec<serde_json::Value> = cheats.iter().map(|c| {
        serde_json::json!({ "code": c.code_str, "enabled": c.enabled })
    }).collect();
    if let Ok(data) = serde_json::to_string_pretty(&entries) {
        let _ = fs::write(dir.join(format!("{}.json", rom_name)), data);
    }
}

fn load_cheats(rom_name: &str) -> Vec<oxidenes::bus::GameGenieCode> {
    if rom_name.is_empty() { return Vec::new(); }
    let path = cheats_dir().join(format!("{}.json", rom_name));
    let Ok(data) = fs::read_to_string(&path) else { return Vec::new(); };
    let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&data) else { return Vec::new(); };
    entries.iter().filter_map(|e| {
        let code_str = e.get("code")?.as_str()?;
        let enabled = e.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
        let mut code = oxidenes::bus::GameGenieCode::decode(code_str)?;
        code.enabled = enabled;
        Some(code)
    }).collect()
}

fn save_state_path(config: &EmulatorConfig, slot: u8) -> Option<PathBuf> {
    let recent = config.recent_games.first()?;
    let filename = Path::new(recent).file_stem()?.to_string_lossy().to_string();
    if slot <= 1 {
        Some(save_state_dir().join(format!("{}.sav", filename)))
    } else {
        Some(save_state_dir().join(format!("{}.sav{}", filename, slot)))
    }
}

fn capture_thumbnail(frame_data: &[u32]) -> Vec<u8> {
    let mut thumb = Vec::with_capacity(64 * 60 * 3);
    for y in (0..240).step_by(4) {
        for x in (0..256).step_by(4) {
            let pixel = frame_data[y * 256 + x];
            thumb.push(((pixel >> 16) & 0xFF) as u8);
            thumb.push(((pixel >> 8) & 0xFF) as u8);
            thumb.push((pixel & 0xFF) as u8);
        }
    }
    thumb
}

fn save_thumbnail(config: &EmulatorConfig, slot: u8, frame_data: &[u32]) {
    if let Some(path) = save_state_path(config, slot) {
        let thumb_path = PathBuf::from(format!("{}.thumb", path.display()));
        let thumb = capture_thumbnail(frame_data);
        let _ = fs::write(&thumb_path, &thumb);
    }
}

fn load_thumbnail(config: &EmulatorConfig, slot: u8) -> Option<Vec<u8>> {
    let path = save_state_path(config, slot)?;
    let thumb_path = PathBuf::from(format!("{}.thumb", path.display()));
    let data = fs::read(&thumb_path).ok()?;
    if data.len() == 64 * 60 * 3 { Some(data) } else { None }
}

fn save_state(bus: &Bus, cpu: &Cpu, config: &EmulatorConfig, slot: u8) -> bool {
    let path_opt = save_state_path(config, slot);
    let Some(path) = path_opt else { return false; };
    let _ = fs::create_dir_all(save_state_dir());
    
    let mut data = Vec::new();
    // Magic header + version
    data.extend_from_slice(b"NESSAV02");
    
    // CPU state
    let cpu_state = cpu.save_state();
    data.extend_from_slice(&(cpu_state.len() as u32).to_le_bytes());
    data.extend(cpu_state);
    
    // Bus state (RAM + PPU + mapper)
    let bus_state = bus.save_state();
    data.extend_from_slice(&(bus_state.len() as u32).to_le_bytes());
    data.extend(bus_state);
    
    // Save thumbnail alongside state
    save_thumbnail(config, slot, &bus.ppu.frame_data);
    
    fs::write(&path, &data).is_ok()
}

fn load_state(bus: &mut Bus, cpu: &mut Cpu, config: &EmulatorConfig, slot: u8) -> bool {
    let path_opt = save_state_path(config, slot);
    let Some(path) = path_opt else { return false; };
    if !path.exists() { return false; }
    let Ok(data) = fs::read(&path) else { return false; };
    
    // Check magic header (accept V02 for backward compat, V03 with ROM fingerprint)
    if data.len() < 8 { return false; }
    let is_v03 = &data[0..8] == b"NESSAV03";
    let is_v02 = &data[0..8] == b"NESSAV02";
    if !is_v02 && !is_v03 { return false; }
    let mut pos = 8;
    
    // V03: skip ROM fingerprint (deprecated — validation was too aggressive)
    if is_v03 {
        if pos + 4 > data.len() { return false; }
        pos += 4; // skip hash, don't validate
    }
    
    // CPU state
    if pos + 4 > data.len() { return false; }
    let cpu_len = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
    pos += 4;
    if pos + cpu_len > data.len() { return false; }
    if !cpu.load_state(&data[pos..pos+cpu_len]) { return false; }
    pos += cpu_len;
    
    // Bus state
    if pos + 4 > data.len() { return false; }
    let bus_len = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
    pos += 4;
    if pos + bus_len > data.len() { return false; }
    if !bus.load_state(&data[pos..pos+bus_len]) { return false; }
    
    true
}

// =====================================================================
// Rewind support (hold Backspace to rewind)
// =====================================================================

struct RewindBuffer {
    snapshots: VecDeque<Vec<u8>>,
    max_snapshots: usize,
    max_bytes: usize,
    total_bytes: usize,
    frame_skip: u32,
    frame_counter: u32,
}

impl RewindBuffer {
    fn new() -> Self {
        RewindBuffer {
            snapshots: VecDeque::new(),
            max_snapshots: 300,
            max_bytes: 64 * 1024 * 1024, // 64MB cap (includes PPU frame data for smooth rewind)
            total_bytes: 0,
            frame_skip: 4,     // save every 4th frame (~15 snapshots/sec)
            frame_counter: 0,
        }
    }

    fn push_frame(&mut self, bus: &Bus, cpu: &Cpu) {
        self.frame_counter += 1;
        if self.frame_counter % self.frame_skip != 0 {
            return;
        }
        
        let mut snapshot = Vec::new();
        let cpu_state = cpu.save_state();
        snapshot.extend_from_slice(&(cpu_state.len() as u32).to_le_bytes());
        snapshot.extend(cpu_state);
        let bus_state = bus.save_state();
        snapshot.extend_from_slice(&(bus_state.len() as u32).to_le_bytes());
        snapshot.extend(bus_state);
        // Store PPU frame buffer for smooth rewind playback
        let frame = &bus.ppu.frame_data;
        snapshot.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        for pixel in frame.iter() {
            snapshot.extend_from_slice(&pixel.to_le_bytes());
        }
        
        self.total_bytes += snapshot.len();
        self.snapshots.push_back(snapshot);
        
        // Cap at max_snapshots AND max_bytes (64MB default)
        while self.snapshots.len() > self.max_snapshots || self.total_bytes > self.max_bytes {
            if let Some(old) = self.snapshots.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(old.len());
            } else {
                break;
            }
        }
    }

    fn pop_frame(&mut self, bus: &mut Bus, cpu: &mut Cpu) -> bool {
        if let Some(snapshot) = self.snapshots.pop_back() {
            self.total_bytes -= snapshot.len();
            let mut pos = 0;
            if pos + 4 > snapshot.len() { return false; }
            let cpu_len = u32::from_le_bytes([snapshot[pos], snapshot[pos+1], snapshot[pos+2], snapshot[pos+3]]) as usize;
            pos += 4;
            if pos + cpu_len > snapshot.len() { return false; }
            if !cpu.load_state(&snapshot[pos..pos+cpu_len]) { return false; }
            pos += cpu_len;
            
            if pos + 4 > snapshot.len() { return false; }
            let bus_len = u32::from_le_bytes([snapshot[pos], snapshot[pos+1], snapshot[pos+2], snapshot[pos+3]]) as usize;
            pos += 4;
            if pos + bus_len > snapshot.len() { return false; }
            if !bus.load_state(&snapshot[pos..pos+bus_len]) { return false; }
            pos += bus_len;
            // Restore PPU frame buffer for smooth rewind playback
            if pos + 4 <= snapshot.len() {
                let frame_len = u32::from_le_bytes([snapshot[pos], snapshot[pos+1], snapshot[pos+2], snapshot[pos+3]]) as usize;
                pos += 4;
                if pos + frame_len * 4 <= snapshot.len() {
                    bus.ppu.frame_data.resize(frame_len, 0);
                    for i in 0..frame_len {
                        let off = pos + i * 4;
                        bus.ppu.frame_data[i] = u32::from_le_bytes([
                            snapshot[off], snapshot[off+1], snapshot[off+2], snapshot[off+3]
                        ]);
                    }
                }
            }
            true
        } else {
            false
        }
    }
    
    fn clear(&mut self) {
        self.snapshots.clear();
        self.total_bytes = 0;
        self.frame_counter = 0;
    }
}

// =====================================================================
// Battery SRAM persistence (automatic save for Zelda, FF, etc.)
// =====================================================================

fn sram_path(config: &EmulatorConfig) -> Option<PathBuf> {
    let recent = config.recent_games.first()?;
    let filename = Path::new(recent).file_stem()?.to_string_lossy().to_string();
    Some(save_state_dir().join(format!("{}.sram", filename)))
}

fn auto_save_sram(bus: &Bus, config: &EmulatorConfig) {
    if !bus.cartridge.has_battery { return; }
    let Some(path) = sram_path(config) else { return; };
    let _ = fs::create_dir_all(save_state_dir());
    let sram = bus.get_sram();
    if !sram.is_empty() {
        let _ = fs::write(&path, &sram);
    }
}

fn auto_load_sram(bus: &mut Bus, config: &EmulatorConfig) {
    if !bus.cartridge.has_battery { return; }
    let Some(path) = sram_path(config) else { return; };
    if let Ok(data) = fs::read(&path) {
        bus.set_sram(&data);
    }
}

// =====================================================================
// Screenshot support
// =====================================================================

fn screenshot_path() -> PathBuf {
    let dir = config_dir().join("screenshots");
    let _ = fs::create_dir_all(&dir);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    dir.join(format!("nes_{}.ppm", timestamp))
}

fn save_screenshot(frame_data: &[u32]) -> Option<String> {
    let path = screenshot_path();
    let data = format!("P6\n256 240\n255\n");
    let mut bytes = data.into_bytes();
    for &pixel in frame_data.iter().take(256 * 240) {
        bytes.push(((pixel >> 16) & 0xFF) as u8);
        bytes.push(((pixel >> 8) & 0xFF) as u8);
        bytes.push((pixel & 0xFF) as u8);
    }
    if fs::write(&path, &bytes).is_ok() {
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
}

// =====================================================================
// Emulator state machine
// =====================================================================

enum EmulatorState {
    Menu(MenuState),
    Game,
}

struct MenuState {
    selected: usize,
    submenu: Option<SubMenu>,
    cursor_visible: bool,
    cursor_timer: u32,
    // Screen transition fade effect
    transition_timer: u8,      // counts down from 6 to 0
    transition_out: bool,      // true = fading out, false = fading in
    favorites_page: usize,
}

impl MenuState {
    fn new() -> Self {
        Self {
            selected: 0,
            submenu: None,
            cursor_visible: true,
            cursor_timer: 0,
            transition_timer: 0,
            transition_out: false,
            favorites_page: 0,
        }
    }
}

enum SubMenu {
    Settings { selected: usize, value_flash: u8 },
    FileBrowser(FileBrowser),
    InputSettings(InputSettingsState),
    CrtSettings { selected: usize, tables_dirty: bool, value_flash: u8 },
    FolderSetup { browser: FileBrowser, from_settings: bool },
}

struct InputSettingsState {
    tab: u8,  // 0=KB P1, 1=KB P2, 2=Ctrl P1, 3=Ctrl P2
    selected: usize,
    waiting_for_input: bool,
    bindings: InputBindings,  // working copy
    conflict_message: Option<String>,
    conflict_timer: u32,
}

struct FileBrowserEntry {
    name: String,
    is_dir: bool,
    full_path: PathBuf,
    size_kb: u32,
}

struct FileBrowser {
    current_dir: PathBuf,
    entries: Vec<FileBrowserEntry>,
    selected: usize,
    scroll_offset: usize,
    error_message: Option<String>,
    error_timer: u32,
}

impl FileBrowser {
    fn new(start_dir: Option<&str>) -> Self {
        let dir = if let Some(d) = start_dir {
            let p = PathBuf::from(d);
            if p.is_dir() {
                p
            } else {
                Self::default_dir()
            }
        } else {
            Self::default_dir()
        };
        let entries = scan_directory(&dir);
        FileBrowser {
            current_dir: dir,
            entries,
            selected: 0,
            scroll_offset: 0,
            error_message: None,
            error_timer: 0,
        }
    }

    fn default_dir() -> PathBuf {
        let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
        let roms_dir = PathBuf::from(&home).join(".nes-emulator").join("roms");
        let downloads = PathBuf::from(&home).join("Downloads");
        if roms_dir.is_dir() {
            roms_dir
        } else if downloads.is_dir() {
            downloads
        } else {
            PathBuf::from(".")
        }
    }

    fn navigate_to(&mut self, dir: &Path) {
        match scan_directory_result(dir) {
            Ok(entries) => {
                self.current_dir = dir.to_path_buf();
                self.entries = entries;
                self.selected = 0;
                self.scroll_offset = 0;
                self.error_message = None;
                self.error_timer = 0;
            }
            Err(_) => {
                self.error_message = Some("ACCESS DENIED".to_string());
                self.error_timer = 180;
            }
        }
    }
}

fn scan_directory(dir: &Path) -> Vec<FileBrowserEntry> {
    scan_directory_result(dir).unwrap_or_default()
}

fn scan_directory_result(dir: &Path) -> Result<Vec<FileBrowserEntry>, std::io::Error> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            dirs.push(FileBrowserEntry {
                name,
                is_dir: true,
                full_path: path,
                size_kb: 0,
            });
        } else if name.to_lowercase().ends_with(".nes") {
            let size_kb = if path.is_file() {
                (fs::metadata(&path).map(|m| m.len()).unwrap_or(0) / 1024) as u32
            } else { 0 };
            files.push(FileBrowserEntry {
                name,
                is_dir: false,
                full_path: path,
                size_kb,
            });
        }
    }

    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    files.extend(dirs);
    Ok(files)
}

enum MenuAction {
    LoadRom(String),
}

// =====================================================================
// 8x8 bitmap font
// =====================================================================

fn get_font_glyph(ch: char) -> [u8; 8] {
    match ch {
        'A' => [0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x00],
        'B' => [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
        'C' => [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00],
        'D' => [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00],
        'E' => [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0x00],
        'F' => [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00],
        'G' => [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3E, 0x00],
        'H' => [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
        'I' => [0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
        'J' => [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x38, 0x00],
        'K' => [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00],
        'L' => [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00],
        'M' => [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
        'N' => [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00],
        'O' => [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        'P' => [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
        'Q' => [0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00],
        'R' => [0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x00],
        'S' => [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
        'T' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        'U' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        'V' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
        'W' => [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
        'X' => [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00],
        'Y' => [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00],
        'Z' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00],
        '0' => [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00],
        '1' => [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
        '2' => [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00],
        '3' => [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00],
        '4' => [0x0C, 0x1C, 0x3C, 0x6C, 0x7E, 0x0C, 0x0C, 0x00],
        '5' => [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00],
        '6' => [0x3C, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00],
        '7' => [0x7E, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x18, 0x00],
        '8' => [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00],
        '9' => [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x3C, 0x00],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
        ':' => [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00],
        '/' => [0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
        '\\' => [0x40, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x02, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00],
        '(' => [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00],
        ')' => [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00],
        '%' => [0x62, 0x64, 0x08, 0x10, 0x20, 0x4C, 0x8C, 0x00],
        '=' => [0x00, 0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00],
        '!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30],
        '#' => [0x24, 0x24, 0x7E, 0x24, 0x7E, 0x24, 0x24, 0x00],
        '+' => [0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00],
        '[' => [0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00],
        ']' => [0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00],
        '>' => [0x00, 0x30, 0x18, 0x0C, 0x18, 0x30, 0x00, 0x00],
        '<' => [0x00, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x00, 0x00],
        // Arrow (right-pointing triangle)
        '\x10' => [0x00, 0x40, 0x60, 0x70, 0x78, 0x70, 0x60, 0x40],
        // Star
        '\x11' => [0x10, 0x10, 0x38, 0xFE, 0x38, 0x10, 0x10, 0x00],
        _ => [0x00; 8],
    }
}

// =====================================================================
// Menu rendering (256x240 framebuffer)
// =====================================================================

fn draw_char_8x8(fb: &mut [u32], ch: char, tile_x: usize, tile_y: usize, color: u32) {
    let glyph = get_font_glyph(ch);
    let px = tile_x * 8;
    let py = tile_y * 8;
    for row in 0..8 {
        let bits = glyph[row];
        let y = py + row;
        if y >= 240 { break; }
        for col in 0..8 {
            if bits & (0x80 >> col) != 0 {
                let x = px + col;
                if x < 256 {
                    fb[y * 256 + x] = color;
                }
            }
        }
    }
}

fn draw_text_8x8(fb: &mut [u32], text: &str, tile_x: usize, tile_y: usize, color: u32) {
    for (i, ch) in text.chars().enumerate() {
        draw_char_8x8(fb, ch, tile_x + i, tile_y, color);
    }
}

fn draw_text_centered_8x8(fb: &mut [u32], text: &str, tile_y: usize, color: u32) {
    let len = text.chars().count();
    let tile_x = if len < 32 { (32 - len) / 2 } else { 0 };
    draw_text_8x8(fb, text, tile_x, tile_y, color);
}

fn draw_horizontal_line_px(fb: &mut [u32], y: usize, x_start: usize, x_end: usize, color: u32) {
    if y < 240 {
        for x in x_start..x_end.min(256) {
            fb[y * 256 + x] = color;
        }
    }
}

fn draw_double_border_top(fb: &mut [u32], tile_row: usize) {
    let y_base = tile_row * 8;
    let color = MENU_LIGHT_BLUE;
    draw_horizontal_line_px(fb, y_base + 2, 16, 240, color);
    draw_horizontal_line_px(fb, y_base + 4, 16, 240, color);
    for y in (y_base + 2)..=(y_base + 4 + 8) {
        if y < 240 {
            fb[y * 256 + 16] = color;
            fb[y * 256 + 18] = color;
            fb[y * 256 + 239] = color;
            fb[y * 256 + 237] = color;
        }
    }
}

fn draw_double_border_bottom(fb: &mut [u32], tile_row: usize) {
    let y_base = tile_row * 8;
    let color = MENU_LIGHT_BLUE;
    for y in y_base..(y_base + 4) {
        if y < 240 {
            fb[y * 256 + 16] = color;
            fb[y * 256 + 18] = color;
            fb[y * 256 + 239] = color;
            fb[y * 256 + 237] = color;
        }
    }
    draw_horizontal_line_px(fb, y_base + 3, 16, 240, color);
    draw_horizontal_line_px(fb, y_base + 5, 16, 240, color);
}

fn draw_separator_line(fb: &mut [u32], tile_row: usize) {
    let y = tile_row * 8 + 4;
    for x in 24..232 {
        if x % 4 < 2 {
            if y < 240 {
                fb[y * 256 + x] = MENU_DARK_GRAY;
            }
        }
    }
}

fn draw_side_borders(fb: &mut [u32]) {
    for tile_row in 2..28 {
        let y_base = tile_row * 8;
        for y in y_base..(y_base + 8) {
            if y < 240 {
                fb[y * 256 + 16] = MENU_LIGHT_BLUE;
                fb[y * 256 + 18] = MENU_LIGHT_BLUE;
                fb[y * 256 + 239] = MENU_LIGHT_BLUE;
                fb[y * 256 + 237] = MENU_LIGHT_BLUE;
            }
        }
    }
}

/// Draw a selection highlight bar across a region of the framebuffer.
#[inline]
fn draw_highlight_bar(fb: &mut [u32], y_start: usize, height: usize, x_left: usize, x_right: usize, color: u32) {
    for row in y_start..y_start + height {
        if row >= 240 { break; }
        for col in x_left..x_right.min(256) {
            fb[row * 256 + col] = color;
        }
    }
}

/// Apply a fade overlay to the framebuffer for screen transitions.
/// fade_level: 0=full brightness, 8=nearly black
#[inline]
fn apply_menu_fade(fb: &mut [u32], width: usize, height: usize, fade_level: u8) {
    if fade_level == 0 { return; }
    // fade_level 0=full brightness, 8=nearly black
    let brightness = (255u32).saturating_sub(fade_level as u32 * 30); // 255, 225, 195... down to 15
    for pixel in fb[..width * height].iter_mut() {
        let r = ((*pixel >> 16) & 0xFF) * brightness / 255;
        let g = ((*pixel >> 8) & 0xFF) * brightness / 255;
        let b = (*pixel & 0xFF) * brightness / 255;
        *pixel = (r << 16) | (g << 8) | b;
    }
}

fn render_home_screen(fb: &mut [u32], menu: &MenuState, cfg: &EmulatorConfig, cursor_visible: bool, favorites_valid: &[bool], recents_valid: &[bool]) {
    for pixel in fb.iter_mut() { *pixel = MENU_BG; }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 OXIDENES \x11", 2, MENU_GOLD);
    draw_separator_line(fb, 3);

    let mut current_row: usize = 4;
    let mut item_index: usize = 0;

    // === FAVORITES SECTION ===
    let valid_favorites: Vec<&String> = cfg.favorite_games.iter().enumerate()
        .filter(|(i, _)| favorites_valid.get(*i).copied().unwrap_or(false))
        .map(|(_, p)| p)
        .collect();
    let total_favs = valid_favorites.len();
    let per_page = 5usize;
    let total_pages = if total_favs == 0 { 0 } else { (total_favs + per_page - 1) / per_page };
    let page = menu.favorites_page.min(total_pages.saturating_sub(1));
    let page_start = page * per_page;
    let page_end = (page_start + per_page).min(total_favs);
    let fav_count = page_end - page_start;

    if total_favs > 0 {
        // Header with page indicator
        if total_pages > 1 {
            let header = format!("\x11 FAVORITES  {}/{}", page + 1, total_pages);
            draw_text_8x8(fb, &header, 3, current_row, MENU_GOLD);
            // Page hint on right side
            draw_text_8x8(fb, "SEL:\x1A", 25, current_row, MENU_DARK_GRAY);
        } else {
            draw_text_8x8(fb, "\x11 FAVORITES", 3, current_row, MENU_GOLD);
        }
        current_row += 1;

        for i in 0..fav_count {
            let row = current_row;
            if row >= 24 { break; }

            let path_str = valid_favorites[page_start + i];
            let filename = Path::new(path_str)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.clone());

            let is_selected = menu.selected == item_index;

            if is_selected {
                draw_highlight_bar(fb, row * 8, 8, 20, 236, 0x3C3C8C);
            }

            let color = if is_selected { MENU_WHITE } else { MENU_GOLD };

            if is_selected && cursor_visible {
                draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
            }

            let display_name = filename.to_uppercase();
            let display_name = display_name.strip_suffix(".NES").unwrap_or(&display_name);
            let display = format!("\x11 {}", &display_name.chars().take(24).collect::<String>());
            draw_text_8x8(fb, &display, 3, row, color);

            item_index += 1;
            current_row += 1;
        }
        draw_separator_line(fb, current_row);
        current_row += 1;
    }

    // === RECENT GAMES SECTION ===
    let recent_non_fav: Vec<(usize, &String)> = cfg.recent_games.iter().enumerate()
        .filter(|(_, p)| !cfg.favorite_games.contains(p))
        .collect();
    let max_recent = (23_usize.saturating_sub(current_row)).min(8);
    let recent_count = recent_non_fav.len().min(max_recent);

    if recent_count > 0 || fav_count == 0 {
        draw_text_8x8(fb, "RECENT GAMES", 3, current_row, MENU_DARK_GRAY);
        current_row += 1;

        if recent_count == 0 {
            draw_text_centered_8x8(fb, "NO RECENT GAMES YET", current_row, MENU_DARK_GRAY);
            current_row += 1;
            draw_text_centered_8x8(fb, "BROWSE TO PLAY!", current_row, MENU_DARK_GRAY);
            current_row += 1;
        } else {
            for i in 0..recent_count {
                let row = current_row;
                if row >= 24 { break; }

                let (orig_idx, path_str) = recent_non_fav[i];
                let filename = Path::new(path_str)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_str.clone());

                let exists = recents_valid.get(orig_idx).copied().unwrap_or(false);
                let is_selected = menu.selected == item_index;

                if is_selected {
                    draw_highlight_bar(fb, row * 8, 8, 20, 236, 0x3C3C8C);
                }

                let color = if !exists {
                    MENU_DARK_GRAY
                } else if is_selected {
                    MENU_WHITE
                } else {
                    MENU_GRAY
                };

                if is_selected && cursor_visible {
                    draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
                }

                let display_name = filename.to_uppercase();
                let display_name = display_name.strip_suffix(".NES").unwrap_or(&display_name);
                let display: String = display_name.chars().take(26).collect();
                draw_text_8x8(fb, &display, 3, row, color);

                item_index += 1;
                current_row += 1;
            }
        }
        draw_separator_line(fb, current_row);
        current_row += 1;
    }

    // BROWSE FILES option
    {
        let row = current_row;
        let is_selected = menu.selected == item_index;
        if is_selected {
            draw_highlight_bar(fb, row * 8, 8, 20, 236, 0x3C3C8C);
        }
        let color = if is_selected { MENU_WHITE } else { MENU_GRAY };
        if is_selected && cursor_visible {
            draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
        }
        draw_text_8x8(fb, "BROWSE FILES", 3, row, color);
        item_index += 1;
        current_row += 1;
    }

    // SETTINGS option
    {
        let row = current_row;
        let is_selected = menu.selected == item_index;
        if is_selected {
            draw_highlight_bar(fb, row * 8, 8, 20, 236, 0x3C3C8C);
        }
        let color = if is_selected { MENU_WHITE } else { MENU_GRAY };
        if is_selected && cursor_visible {
            draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
        }
        draw_text_8x8(fb, "SETTINGS", 3, row, color);
    }

    draw_separator_line(fb, 24);
    draw_text_centered_8x8(fb, "A:OPEN  F:FAV  ESC:QUIT", 25, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "IN GAME: START+SEL 1s", 26, MENU_DARK_GRAY);

    // Bottom hint
    draw_text_8x8(fb, "DROP .NES ON EXE OR BROWSE", 3, 27, 0x585858);
}

fn render_settings(fb: &mut [u32], cfg: &EmulatorConfig, selected: usize, cursor_visible: bool, audio_volume: u32, glass_intensity: u8, value_flash: u8) {
    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_8x8(fb, "MENU >", 3, 2, 0x666666);
    draw_text_8x8(fb, "SETTINGS", 10, 2, 0xCCCCCC);
    draw_text_centered_8x8(fb, "\x11 SETTINGS \x11", 4, MENU_GOLD);
    draw_separator_line(fb, 5);

    let rom_folder_display = if let Some(ref dir) = cfg.rom_directory {
        let s = dir.replace('\\', "/").to_uppercase();
        if s.len() > 17 { format!("...{}", &s[s.len()-14..]) } else { s }
    } else {
        "NOT SET".to_string()
    };

    let settings_items = [
        format!("CRT FILTER: {}", if cfg.crt_enabled { "ON" } else { "OFF" }),
        format!("BARREL DISTORTION: {}", if cfg.barrel_distortion { "ON" } else { "OFF" }),
        format!("GLASS INTENSITY: {}%", glass_intensity),
        format!("AUDIO VOLUME: {}%", audio_volume),
        format!("REGION: {}", if cfg.region == "pal" { "PAL" } else { "NTSC" }),
        "CRT SETTINGS >".to_string(),
        "INPUT SETTINGS >".to_string(),
        format!("CHECK FOR UPDATES: {}", if cfg.check_for_updates { "ON" } else { "OFF" }),
        format!("ROM FOLDER: {}", rom_folder_display),
    ];
    let setting_rows = [7, 9, 11, 13, 15, 17, 19, 21, 23];

    for (i, (item, &row)) in settings_items.iter().zip(setting_rows.iter()).enumerate() {
        let is_flashing = i == selected && value_flash > 0;
        let color = if is_flashing { MENU_GOLD } else if i == selected { MENU_WHITE } else { MENU_GRAY };
        if i == selected && cursor_visible {
            draw_char_8x8(fb, '\x10', 3, row, MENU_WHITE);
        }
        draw_text_8x8(fb, item, 5, row, color);
    }

    draw_separator_line(fb, 25);

    draw_text_centered_8x8(fb, "ENTER/LEFT/RIGHT TO CHANGE", 26, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "ESC TO GO BACK", 27, MENU_DARK_GRAY);
}

fn render_crt_settings(fb: &mut [u32], cfg: &EmulatorConfig, selected: usize, cursor_visible: bool, value_flash: u8) {
    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_8x8(fb, "MENU > SETTINGS >", 1, 2, 0x666666);
    draw_text_8x8(fb, "CRT", 19, 2, 0xCCCCCC);
    draw_text_centered_8x8(fb, "\x11 CRT SETTINGS \x11", 4, MENU_GOLD);
    draw_separator_line(fb, 5);

    let crt = &cfg.crt_config;
    let mask_name = match crt.mask_mode {
        CrtMaskMode::Off => "OFF",
        CrtMaskMode::ShadowMask => "SHADOW MASK",
        CrtMaskMode::ApertureGrille => "APERTURE GRILLE",
        CrtMaskMode::SlotMask => "SLOT MASK",
    };

    let items: [(& str, String); 9] = [
        ("SCANLINES:", format_slider_bar(crt.scanline_intensity)),
        ("PHOSPHOR:", format_slider_bar(crt.phosphor_warmth)),
        ("VIGNETTE:", format_slider_bar(crt.vignette_strength)),
        ("BLUR:", format_slider_bar(crt.blur_amount)),
        ("CURVATURE:", format_slider_bar(crt.curvature_strength)),
        ("GLASS:", format_slider_bar(cfg.glass_intensity)),
        ("MASK:", mask_name.to_string()),
        ("MASK INT:", format_slider_bar(crt.mask_intensity)),
        ("BACK", String::new()),
    ];
    let rows = [7, 9, 11, 13, 15, 17, 19, 21, 23];

    for (i, ((label, value), &row)) in items.iter().zip(rows.iter()).enumerate() {
        let is_flashing = i == selected && value_flash > 0;
        let color = if is_flashing { MENU_GOLD } else if i == selected { MENU_WHITE } else { MENU_GRAY };
        let value_color = if is_flashing { MENU_GOLD } else { color };
        if i == selected && cursor_visible {
            draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
        }
        draw_text_8x8(fb, label, 4, row, color);
        if !value.is_empty() {
            draw_text_8x8(fb, value, 16, row, value_color);
        }
    }

    draw_separator_line(fb, 24);
    draw_text_centered_8x8(fb, "LEFT/RIGHT TO ADJUST", 25, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "ESC/BACK TO RETURN", 26, MENU_DARK_GRAY);
}

fn format_slider_bar(value: u8) -> String {
    let filled = (value as usize * 20) / 100;
    let empty = 20 - filled;
    // Use simple ASCII chars for the bar (compatible with 8x8 font)
    let bar: String = "#".repeat(filled) + &"-".repeat(empty);
    format!("[{}] {}%", bar, value)
}

fn render_input_settings(fb: &mut [u32], state: &InputSettingsState, cursor_visible: bool) {
    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 INPUT SETTINGS \x11", 2, MENU_GOLD);
    draw_text_8x8(fb, "MENU > SETTINGS >", 1, 3, 0x666666);
    draw_text_8x8(fb, "INPUT", 19, 3, 0xCCCCCC);

    // Tab headers
    let tabs = ["KB P1", "KB P2", "PAD P1", "PAD P2"];
    let mut tab_x = 4;
    for (i, tab) in tabs.iter().enumerate() {
        let color = if i == state.tab as usize { MENU_LIGHT_BLUE } else { MENU_DARK_GRAY };
        draw_text_8x8(fb, &format!("[{}]", tab), tab_x, 4, color);
        tab_x += 8;
    }

    draw_separator_line(fb, 5);

    // Binding lists based on active tab
    let current_row = 7;
    
    match state.tab {
        0 | 1 => {
            // Keyboard bindings
            let bindings = if state.tab == 0 { &state.bindings.keyboard_p1 } else { &state.bindings.keyboard_p2 };
            let binding_names = ["UP", "DOWN", "LEFT", "RIGHT", "A", "B", "START", "SELECT", "TURBO A", "TURBO B"];
            let binding_values = [
                &bindings.up, &bindings.down, &bindings.left, &bindings.right,
                &bindings.a, &bindings.b, &bindings.start, &bindings.select,
                &bindings.turbo_a, &bindings.turbo_b
            ];
            
            for (i, (name, value)) in binding_names.iter().zip(binding_values.iter()).enumerate() {
                let row = current_row + i;
                let color = if i == state.selected { MENU_WHITE } else { MENU_GRAY };
                
                if i == state.selected && cursor_visible {
                    draw_char_8x8(fb, '\x10', 3, row, MENU_WHITE);
                }
                
                if state.waiting_for_input && i == state.selected {
                    draw_text_8x8(fb, &format!("{}:", name), 5, row, color);
                    draw_text_8x8(fb, "PRESS A KEY...", 18, row, MENU_GOLD);
                } else {
                    draw_text_8x8(fb, &format!("{}:", name), 5, row, color);
                    draw_text_8x8(fb, value, 18, row, color);
                }
            }
        }
        2 | 3 => {
            // Controller bindings
            let bindings = if state.tab == 2 { &state.bindings.controller_p1 } else { &state.bindings.controller_p2 };
            let binding_names = ["A", "B", "TURBO A", "TURBO B", "START", "SELECT", "DEADZONE"];
            let binding_values = [
                bindings.a.as_str(), bindings.b.as_str(), bindings.turbo_a.as_str(), 
                bindings.turbo_b.as_str(), bindings.start.as_str(), bindings.select.as_str(),
                ""  // Special case for deadzone
            ];
            
            for (i, (name, value)) in binding_names.iter().zip(binding_values.iter()).enumerate() {
                let row = current_row + i;
                let color = if i == state.selected { MENU_WHITE } else { MENU_GRAY };
                
                if i == state.selected && cursor_visible {
                    draw_char_8x8(fb, '\x10', 3, row, MENU_WHITE);
                }
                
                if i == 6 {
                    // Deadzone special handling
                    draw_text_8x8(fb, &format!("{}:", name), 5, row, color);
                    draw_text_8x8(fb, &format!("{:.2}", bindings.deadzone), 18, row, color);
                } else if state.waiting_for_input && i == state.selected {
                    draw_text_8x8(fb, &format!("{}:", name), 5, row, color);
                    draw_text_8x8(fb, "PRESS A BUTTON...", 18, row, MENU_GOLD);
                } else {
                    draw_text_8x8(fb, &format!("{}:", name), 5, row, color);
                    draw_text_8x8(fb, value, 18, row, color);
                }
            }
        }
        _ => {}
    }

    // Conflict message
    if let Some(ref message) = state.conflict_message {
        draw_text_centered_8x8(fb, message, 19, 0xFF4444);
    }

    draw_separator_line(fb, 21);
    draw_text_centered_8x8(fb, "Enter: REBIND    Tab: NEXT TAB", 22, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "Esc: BACK (save)", 23, MENU_DARK_GRAY);
}

fn truncate_path_display(path: &Path, max_chars: usize) -> String {
    let s = path.to_string_lossy()
        .to_uppercase()
        .replace('\\', "/");
    if s.len() <= max_chars {
        s
    } else {
        format!("...{}", &s[s.len() - (max_chars - 3)..])
    }
}

fn render_file_browser(fb: &mut [u32], browser: &FileBrowser, cursor_visible: bool, cfg: &EmulatorConfig) {
    const VISIBLE_ROWS: usize = 20;
    const FIRST_ROW: usize = 5;
    const DIR_COLOR: u32 = 0x5C94FC;
    const DIR_COLOR_SEL: u32 = 0x7CB4FC;
    const HIGHLIGHT_BG: u32 = 0x3C3C8C;

    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 LOAD CARTRIDGE \x11", 2, MENU_GOLD);

    let path_str = truncate_path_display(&browser.current_dir, 26);
    draw_text_8x8(fb, &path_str, 3, 3, MENU_DARK_GRAY);

    draw_separator_line(fb, 4);

    if browser.entries.is_empty() {
        draw_text_centered_8x8(fb, "NO FILES FOUND", 14, MENU_DARK_GRAY);
    } else {
        let start = browser.scroll_offset;
        let end = (start + VISIBLE_ROWS).min(browser.entries.len());

        for i in start..end {
            let row = FIRST_ROW + (i - start);
            let entry = &browser.entries[i];
            let is_selected = i == browser.selected;

            let name_upper = entry.name.to_uppercase();
            let display_name = if name_upper.len() > 24 {
                format!("{}...", &name_upper[..21])
            } else {
                name_upper
            };

            let is_fav = !entry.is_dir && is_favorite(cfg, &entry.full_path.to_string_lossy());

            let display = if entry.is_dir {
                format!("> {}", display_name)
            } else if is_fav {
                format!("\x11 {}", display_name)
            } else {
                format!("  {}", display_name)
            };

            if is_selected {
                // Highlight bar
                draw_highlight_bar(fb, row * 8, 8, 20, 236, HIGHLIGHT_BG);
                if cursor_visible {
                    draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
                }
                let color = if entry.is_dir { DIR_COLOR_SEL } else { MENU_WHITE };

                if !entry.is_dir && entry.size_kb > 0 {
                    let size_str = format!("{}K", entry.size_kb);
                    let size_x = 28 - size_str.len().min(6);
                    // Truncate display name so it doesn't overlap with size
                    let max_name_chars = size_x.saturating_sub(3);
                    let truncated = if display.len() > max_name_chars && max_name_chars > 3 {
                        format!("{}...", &display[..max_name_chars - 3])
                    } else {
                        display.clone()
                    };
                    draw_text_8x8(fb, &truncated, 3, row, color);
                    draw_text_8x8(fb, &size_str, size_x, row, MENU_DARK_GRAY);
                } else {
                    draw_text_8x8(fb, &display, 3, row, color);
                }
            } else {
                let color = if entry.is_dir { DIR_COLOR } else { MENU_GRAY };
                draw_text_8x8(fb, &display, 3, row, color);
            }
        }

        // Scroll position indicator (top-right corner)
        if browser.entries.len() > VISIBLE_ROWS {
            let pos_str = format!("{}/{}", browser.selected + 1, browser.entries.len());
            let pos_x = 28 - pos_str.len().min(8);
            draw_text_8x8(fb, &pos_str, pos_x, 3, MENU_DARK_GRAY);
        }
    }

    draw_separator_line(fb, 25);
    draw_text_centered_8x8(fb, "A:OPEN B:BACK F:FAV L/R:PG", 26, MENU_DARK_GRAY);

    // Error overlay
    if let Some(ref msg) = browser.error_message {
        let msg_upper = msg.to_uppercase();
        let box_row = 13;
        for x in 40..216 {
            for dy in 0..24 {
                let y = box_row * 8 + dy;
                if y < 240 {
                    fb[y * 256 + x] = 0x442200;
                }
            }
        }
        draw_text_centered_8x8(fb, &msg_upper, box_row + 1, 0xFFCC44);
    }
}

fn render_folder_setup(fb: &mut [u32], browser: &FileBrowser, cursor_visible: bool) {
    const VISIBLE_ROWS: usize = 14;
    const FIRST_ROW: usize = 9;
    const DIR_COLOR: u32 = 0x5C94FC;
    const DIR_COLOR_SEL: u32 = 0x7CB4FC;
    const HIGHLIGHT_BG: u32 = 0x3C3C8C;

    for pixel in fb.iter_mut() { *pixel = MENU_BG; }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 OXIDENES \x11", 2, MENU_GOLD);
    draw_separator_line(fb, 3);

    draw_text_centered_8x8(fb, "WELCOME!", 4, MENU_WHITE);
    draw_text_centered_8x8(fb, "SELECT YOUR ROM FOLDER", 6, MENU_GRAY);

    let path_str = truncate_path_display(&browser.current_dir, 28);
    draw_text_8x8(fb, &path_str, 2, 7, MENU_GOLD);

    draw_separator_line(fb, 8);

    if browser.entries.is_empty() {
        draw_text_centered_8x8(fb, "EMPTY FOLDER", 15, MENU_DARK_GRAY);
    } else {
        let start = browser.scroll_offset;
        let end = (start + VISIBLE_ROWS).min(browser.entries.len());

        for i in start..end {
            let row = FIRST_ROW + (i - start);
            let entry = &browser.entries[i];
            let is_selected = i == browser.selected;

            let name_upper = entry.name.to_uppercase();
            let display_name = if name_upper.len() > 24 {
                format!("{}...", &name_upper[..21])
            } else {
                name_upper
            };

            let display = if entry.is_dir {
                format!("> {}", display_name)
            } else {
                format!("  {}", display_name)
            };

            if is_selected {
                draw_highlight_bar(fb, row * 8, 8, 20, 236, HIGHLIGHT_BG);
                if cursor_visible {
                    draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
                }
                let color = if entry.is_dir { DIR_COLOR_SEL } else { MENU_DARK_GRAY };
                draw_text_8x8(fb, &display, 3, row, color);
            } else {
                let color = if entry.is_dir { DIR_COLOR } else { MENU_DARK_GRAY };
                draw_text_8x8(fb, &display, 3, row, color);
            }
        }

        // Scroll position indicator
        if browser.entries.len() > VISIBLE_ROWS {
            let pos_str = format!("{}/{}", browser.selected + 1, browser.entries.len());
            let pos_x = 28 - pos_str.len().min(8);
            draw_text_8x8(fb, &pos_str, pos_x, 7, MENU_DARK_GRAY);
        }
    }

    draw_separator_line(fb, 23);
    draw_text_centered_8x8(fb, "A:OPEN  B:PARENT", 24, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "TAB: USE THIS FOLDER", 25, MENU_GOLD);

    // .nes file count hint
    let nes_count = browser.entries.iter().filter(|e| !e.is_dir).count();
    if nes_count > 0 {
        let hint = format!("{} .NES FILES HERE", nes_count);
        draw_text_centered_8x8(fb, &hint, 26, MENU_DARK_GRAY);
    }

    // Error overlay
    if let Some(ref msg) = browser.error_message {
        let msg_upper = msg.to_uppercase();
        let box_row = 13;
        for x in 40..216 {
            for dy in 0..24 {
                let y = box_row * 8 + dy;
                if y < 240 {
                    fb[y * 256 + x] = 0x442200;
                }
            }
        }
        draw_text_centered_8x8(fb, &msg_upper, box_row + 1, 0xFFCC44);
    }
}

fn build_flat_distortion_table() -> Vec<(u32, u32)> {
    let mut table = Vec::with_capacity(SCREEN_W * SCREEN_H);
    for dst_y in 0..SCREEN_H {
        for dst_x in 0..SCREEN_W {
            let src_x = dst_x as f32 / SCREEN_W as f32 * 256.0;
            let src_y = dst_y as f32 / SCREEN_H as f32 * 240.0;
            let src_x = src_x.clamp(0.0, 255.98);
            let src_y = src_y.clamp(0.0, 239.98);
            let src_xf = (src_x * 256.0) as u32;
            let src_yf = (src_y * 256.0) as u32;
            table.push((src_xf, src_yf));
        }
    }
    table
}

fn build_vignette_table_with_strength(strength: u8) -> Vec<u16> {
    let mut table = vec![0u16; SCREEN_W * SCREEN_H];
    let scale = strength as f32 / 60.0;
    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            let fx = (x as f32 / SCREEN_W as f32) - 0.5;
            let fy = (y as f32 / SCREEN_H as f32) - 0.5;
            let v = 1.0 - (fx * fx + fy * fy) * 1.5 * scale;
            table[y * SCREEN_W + x] = (v.max(0.3).min(1.0) * 256.0) as u16;
        }
    }
    table
}

fn build_distortion_table_with_curvature(curvature: u8) -> Vec<(u32, u32)> {
    let mut table = Vec::with_capacity(SCREEN_W * SCREEN_H);
    let amount = curvature as f32 / 3333.0;
    for dst_y in 0..SCREEN_H {
        for dst_x in 0..SCREEN_W {
            let nx = (dst_x as f32 / SCREEN_W as f32) * 2.0 - 1.0;
            let ny = (dst_y as f32 / SCREEN_H as f32) * 2.0 - 1.0;
            let r2 = nx * nx + ny * ny;
            let distortion = 1.0 + amount * r2;
            let dx = nx / distortion;
            let dy = ny / distortion;
            let src_x = ((dx + 1.0) / 2.0) * 256.0;
            let src_y = ((dy + 1.0) / 2.0) * 240.0;
            let src_x = src_x.clamp(0.0, 255.98);
            let src_y = src_y.clamp(0.0, 239.98);
            let src_xf = (src_x * 256.0) as u32;
            let src_yf = (src_y * 256.0) as u32;
            table.push((src_xf, src_yf));
        }
    }
    table
}

fn build_mask_table(mode: &CrtMaskMode) -> Vec<(u8, u8, u8)> {
    let mut table = vec![(255u8, 255u8, 255u8); SCREEN_W * SCREEN_H];
    match mode {
        CrtMaskMode::Off => {} // all (255,255,255) — no effect
        CrtMaskMode::ShadowMask => {
            // Shadow mask: 3×2 repeating phosphor triads with half-cell row offset
            // Finer pattern — each dot is 1 output pixel wide
            for y in 0..SCREEN_H {
                let row_in_cell = y % 2;
                let col_offset = if (y / 2) % 2 == 0 { 0 } else { 1 };
                for x in 0..SCREEN_W {
                    let col_in_cell = (x + col_offset) % 3;
                    let (r, g, b) = match (row_in_cell, col_in_cell) {
                        (0, 0) => (255, 180, 180),  // R bright — off channels at 70%
                        (0, 1) => (180, 255, 180),  // G bright
                        (0, 2) => (180, 180, 255),  // B bright
                        (1, 0) => (220, 160, 160),  // R dim
                        (1, 1) => (160, 220, 160),  // G dim
                        (1, 2) => (160, 160, 220),  // B dim
                        _ => (200, 200, 200),
                    };
                    table[y * SCREEN_W + x] = (r, g, b);
                }
            }
        }
        CrtMaskMode::ApertureGrille => {
            // Aperture grille: 3-wide vertical RGB stripes (Trinitron style)
            // Each color stripe is exactly 1 output pixel wide
            for y in 0..SCREEN_H {
                for x in 0..SCREEN_W {
                    let (r, g, b) = match x % 3 {
                        0 => (255, 180, 180),  // R stripe
                        1 => (180, 255, 180),  // G stripe
                        2 => (180, 180, 255),  // B stripe
                        _ => unreachable!(),
                    };
                    table[y * SCREEN_W + x] = (r, g, b);
                }
            }
        }
        CrtMaskMode::SlotMask => {
            // Slot mask: 3-wide RGB groups with vertical offset every 2 rows
            // Most common consumer TV pattern — groups of RGB phosphor triads
            // arranged in a brick-like offset pattern
            for y in 0..SCREEN_H {
                let row_in_cell = y % 3;
                let col_offset = if (y / 3) % 2 == 0 { 0 } else { 2 };
                for x in 0..SCREEN_W {
                    let col_in_cell = (x + col_offset) % 6;
                    let (r, g, b) = match row_in_cell {
                        0 | 1 => {
                            // Active phosphor rows
                            match col_in_cell {
                                0 | 1 => (255, 180, 180),  // R slot
                                2 | 3 => (180, 255, 180),  // G slot
                                4 | 5 => (180, 180, 255),  // B slot
                                _ => unreachable!(),
                            }
                        }
                        2 => {
                            // Dark gap row between slot groups
                            (160, 160, 160)
                        }
                        _ => unreachable!(),
                    };
                    table[y * SCREEN_W + x] = (r, g, b);
                }
            }
        }
    }
    table
}

struct RepeatTracker {
    up_held: u32,
    down_held: u32,
    left_held: u32,
    right_held: u32,
}

impl RepeatTracker {
    fn new() -> Self {
        Self { up_held: 0, down_held: 0, left_held: 0, right_held: 0 }
    }

    fn update_axis(raw_held: bool, counter: &mut u32) -> bool {
        if raw_held {
            *counter += 1;
            if *counter == 1 {
                true // Immediate fire on first press
            } else if *counter <= 20 {
                false // Initial delay: 20 frames (~333ms) — no repeat
            } else if *counter <= 50 {
                // Slow phase: every 6 frames (~10/sec)
                (*counter - 20) % 6 == 0
            } else if *counter <= 90 {
                // Medium phase: every 3 frames (~20/sec)
                (*counter - 50) % 3 == 0
            } else {
                // Fast phase: every 2 frames (~30/sec)
                (*counter - 90) % 2 == 0
            }
        } else {
            *counter = 0;
            false
        }
    }

    fn process(&mut self, raw_up: bool, raw_down: bool, raw_left: bool, raw_right: bool) -> (bool, bool, bool, bool) {
        let up = Self::update_axis(raw_up, &mut self.up_held);
        let down = Self::update_axis(raw_down, &mut self.down_held);
        let left = Self::update_axis(raw_left, &mut self.left_held);
        let right = Self::update_axis(raw_right, &mut self.right_held);
        (up, down, left, right)
    }
}

struct MenuInput {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    confirm: bool,
    back: bool,
    backspace: bool,
    page_up: bool,
    page_down: bool,
    favorite: bool,
    select: bool,
}

fn poll_menu_input(window: &Window, gilrs: &mut Option<Gilrs>, repeat: &mut RepeatTracker, menu_deadzone: f32, stick_state: &mut StickState) -> MenuInput {
    let confirm = window.is_key_pressed(Key::Enter, KeyRepeat::No);
    let back = window.is_key_pressed(Key::Escape, KeyRepeat::No);
    let backspace = window.is_key_pressed(Key::Backspace, KeyRepeat::No);
    let page_up = window.is_key_pressed(Key::PageUp, KeyRepeat::No);
    let page_down = window.is_key_pressed(Key::PageDown, KeyRepeat::No);

    // Raw held state for directional inputs (auto-repeat via RepeatTracker)
    let mut raw_up = window.is_key_down(Key::Up);
    let mut raw_down = window.is_key_down(Key::Down);
    let mut raw_left = window.is_key_down(Key::Left);
    let mut raw_right = window.is_key_down(Key::Right);

    let mut mi = MenuInput {
        up: false, down: false, left: false, right: false,
        confirm, back, backspace, page_up, page_down,
        favorite: window.is_key_pressed(Key::F, KeyRepeat::No),
        select: window.is_key_pressed(Key::Tab, KeyRepeat::No),
    };

    if let Some(ref mut g) = gilrs {
        // Event-driven for one-shot buttons
        while let Some(event) = g.next_event() {
            if let gilrs::EventType::ButtonPressed(btn, _) = event.event {
                match btn {
                    Button::Start | Button::South => mi.confirm = true,
                    Button::East => mi.back = true,
                    Button::West => mi.favorite = true,
                    Button::LeftTrigger | Button::LeftTrigger2 => mi.page_up = true,
                    Button::RightTrigger | Button::RightTrigger2 => mi.page_down = true,
                    Button::Select => mi.select = true,
                    _ => {}
                }
            }
        }
        // State-polled for held directions
        if let Some((_id, gamepad)) = g.gamepads().find(|(_, gp)| gp.is_connected()) {
            raw_up |= gamepad.is_pressed(Button::DPadUp);
            raw_down |= gamepad.is_pressed(Button::DPadDown);
            raw_left |= gamepad.is_pressed(Button::DPadLeft);
            raw_right |= gamepad.is_pressed(Button::DPadRight);
            let stick_x = gamepad.value(Axis::LeftStickX);
            let stick_y = gamepad.value(Axis::LeftStickY);
            let (s_up, s_down, s_left, s_right) = stick_to_dpad(
                stick_x, stick_y, menu_deadzone, stick_state
            );
            raw_up    |= s_up;
            raw_down  |= s_down;
            raw_left  |= s_left;
            raw_right |= s_right;
        }
    }

    let (up, down, left, right) = repeat.process(raw_up, raw_down, raw_left, raw_right);
    mi.up = mi.up || up;
    mi.down = mi.down || down;
    mi.left = mi.left || left;
    mi.right = mi.right || right;

    mi
}

fn generate_menu_tone<P: ringbuf::traits::Producer<Item = f32>>(producer: &mut P, frequency: f32, duration_ms: u32, volume: f32, sample_rate: u32) {
    let num_samples = (sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let envelope = if i < num_samples / 10 {
            i as f32 / (num_samples as f32 / 10.0)
        } else {
            1.0 - (i as f32 - num_samples as f32 / 10.0) / (num_samples as f32 * 9.0 / 10.0)
        };
        let sample = if (t * frequency * 2.0 * std::f32::consts::PI).sin() > 0.0 { volume } else { -volume };
        let _ = producer.try_push(sample * envelope);
    }
}

enum MenuSound {
    Cursor,
    Confirm,
    Back,
    Error,
    Adjust,
}

fn play_menu_sound<P: ringbuf::traits::Producer<Item = f32>>(producer: &mut P, sound: MenuSound, sample_rate: u32, volume: f32) {
    let vol = volume.max(0.3) * 0.15; // minimum 30% so menu is always audible
    match sound {
        MenuSound::Cursor => generate_menu_tone(producer, 880.0, 30, vol, sample_rate),
        MenuSound::Confirm => generate_menu_tone(producer, 440.0, 60, vol, sample_rate),
        MenuSound::Back => generate_menu_tone(producer, 330.0, 40, vol, sample_rate),
        MenuSound::Error => generate_menu_tone(producer, 220.0, 100, vol, sample_rate),
        MenuSound::Adjust => generate_menu_tone(producer, 1200.0, 15, vol, sample_rate),
    }
}

// =====================================================================
// Main function
// =====================================================================

#[cfg(windows)]
fn get_screen_resolution() -> (usize, usize) {
    extern "system" {
        fn GetSystemMetrics(nIndex: i32) -> i32;
    }
    const SM_CXSCREEN: i32 = 0;
    const SM_CYSCREEN: i32 = 1;
    unsafe {
        (GetSystemMetrics(SM_CXSCREEN) as usize, GetSystemMetrics(SM_CYSCREEN) as usize)
    }
}

#[cfg(not(windows))]
fn get_screen_resolution() -> (usize, usize) {
    (1920, 1080)
}

fn main() {
    // CLI flags (handle before any initialization)
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("OxideNES v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("USAGE:");
        println!("    nes-emulator [OPTIONS] [ROM_FILE]");
        println!();
        println!("ARGS:");
        println!("    <ROM_FILE>    Path to a .nes ROM file (optional, opens file browser if omitted)");
        println!();
        println!("OPTIONS:");
        println!("    -h, --help       Show this help message");
        println!("    --version        Show version");
        println!("    --script <FILE>  Load a Lua script on startup");
        println!();
        println!("CONTROLS:");
        println!("    Escape       Pause / Menu");
        println!("    F5           Quick Save");
        println!("    F9           Quick Load");
        println!("    F11          Toggle Fullscreen");
        println!("    Backspace    Rewind (hold)");
        println!("    Tab          Fast Forward (hold)");
        std::process::exit(0);
    }
    if args.contains(&"--version".to_string()) {
        println!("nes-emulator {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    let mut config = load_config();
    let romdb = RomDatabase::new();
    let updater = Updater::new();
    if config.check_for_updates {
        updater.check_async();
    }
    let mut update_dismissed = false;

    let mut window = Window::new(
        &format!("OxideNES v{}", env!("OXIDENES_VERSION")),
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions {
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create window");

    let target_fps = if config.region == "pal" { 50 } else { 60 };
    window.set_target_fps(target_fps);

    // Initialize gamepad support
    let mut gilrs = Gilrs::new().ok();
    if let Some(ref g) = gilrs {
        for (_id, gamepad) in g.gamepads() {
            println!("Controller: {} (connected: {})", gamepad.name(), gamepad.is_connected());
        }
    }

    // Audio ring buffer - lock-free, single producer / single consumer
    let ring = HeapRb::<f32>::new(8192); // ~170ms at 48kHz
    let (mut producer, mut consumer) = ring.split();
    let mut actual_sample_rate = 44100u32;

    let _stream = {
        let host = cpal::default_host();
        let device = host.default_output_device();

        if let Some(device) = device {
            let supported_config = device.default_output_config();
            match supported_config {
                Ok(supported) => {
                    let sample_rate = supported.sample_rate().0;
                    actual_sample_rate = sample_rate;
                    let channels = supported.channels() as usize;

                    let config = cpal::StreamConfig {
                        channels: channels as u16,
                        sample_rate: cpal::SampleRate(sample_rate),
                        buffer_size: cpal::BufferSize::Default,
                    };

                    let mut last_sample: f32 = 0.0;
                    let stream = device.build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            for frame in data.chunks_mut(channels) {
                                let sample = consumer.try_pop().unwrap_or_else(|| {
                                    last_sample *= 0.995;
                                    last_sample
                                });
                                last_sample = sample;
                                for s in frame.iter_mut() {
                                    *s = sample;
                                }
                            }
                        },
                        |err| eprintln!("Audio error: {}", err),
                        None,
                    );

                    match stream {
                        Ok(s) => {
                            let _ = s.play();
                            println!("Audio: {}Hz, {} channels", sample_rate, channels);
                            Some(s)
                        }
                        Err(e) => {
                            eprintln!("Audio disabled: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Audio disabled: {}", e);
                    None
                }
            }
        } else {
            eprintln!("No audio device found");
            None
        }
    };

    // Pre-fill ring buffer to absorb timing jitter
    for _ in 0..800 {
        let _ = producer.try_push(0.0);
    }

    let mut crt_buffer: Vec<u32> = vec![0; SCREEN_W * SCREEN_H];
    let mut ca_temp: Vec<u32> = vec![0; SCREEN_W * SCREEN_H];

    // Build static TV frame once at startup (zero per-frame cost)
    let mut tv_frame_bg = Vec::new();
    build_tv_frame(&mut tv_frame_bg);
    build_console_overlay(&mut tv_frame_bg, TV_HEIGHT, WINDOW_WIDTH, WINDOW_HEIGHT);
    let mut composite_buffer = tv_frame_bg.clone();

    // Pre-compute vignette lookup table (configurable strength)
    let mut vignette_table = build_vignette_table_with_strength(config.crt_config.vignette_strength);
    // Pre-compute barrel distortion lookup table (configurable curvature)
    let mut distortion_table = build_distortion_table_with_curvature(config.crt_config.curvature_strength);
    let flat_distortion_table = build_flat_distortion_table();
    let glare_table = build_glare_table();
    let glass_thickness_table = build_glass_thickness_table();
    let mut mask_table = build_mask_table(&config.crt_config.mask_mode);
    let mut crt_enabled = config.crt_enabled;
    let mut barrel_distortion = config.barrel_distortion;
    let mut audio_volume = config.audio_volume;
    let mut glass_intensity = config.glass_intensity;
    let mut ca_table = build_ca_table(SCREEN_W, SCREEN_H, glass_intensity);
    let mut ghost_alpha_table = build_ghost_alpha_table(glass_intensity);
    let mut ghost_buffer = tv_frame_bg.clone();
    let mut audio_swap_buf: Vec<f32> = Vec::with_capacity(2048);
    // Menu framebuffer (256x240, same as NES PPU output)
    let mut menu_framebuffer = vec![0u32; 256 * 240];

    // State machine
    let mut emulator_state = EmulatorState::Menu(MenuState::new());
    // Show first-run ROM folder setup if not configured
    if config.rom_directory.is_none() {
        if let EmulatorState::Menu(ref mut menu) = emulator_state {
            menu.submenu = Some(SubMenu::FolderSetup {
                browser: FileBrowser::new(None),
                from_settings: false,
            });
        }
    }
    let mut game_bus: Option<Bus> = None;
    let mut game_cpu: Option<Cpu> = None;
    let mut frame_counter: u32 = 0;
    let mut quit_hold_frames: u32 = 0;
    let mut repeat_tracker = RepeatTracker::new();
    let mut overlay_message: Option<String> = None;
    let mut overlay_timer: u32 = 0;
    let mut sound_cooldown: u32 = 0;
    let mut cached_net_text: String = String::new();
    let mut cached_net_ping: u32 = u32::MAX;

    // Fullscreen state
    let mut is_fullscreen: bool = false;
    let mut window_title: String = format!("OxideNES v{}", env!("OXIDENES_VERSION"));

    // Analog stick state for hysteresis
    let mut stick_state_p1 = StickState::default();
    let mut stick_state_p2 = StickState::default();
    let mut stick_state_menu = StickState::default();
    
    // Pause menu state
    let mut paused: bool = false;
    let mut pause_selected: usize = 0;
    let mut pause_cursor_timer: u32 = 0;
    let mut pause_cursor_visible: bool = true;
    // Quick overlay state (L+R menu)
    let mut quick_overlay: bool = false;
    let mut quick_overlay_selected: usize = 0;
    let mut quick_overlay_lr_frames: u8 = 0; // debounce counter
    let mut current_save_slot: u8 = 1;
    let mut pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
    let mut pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
    let mut cheat_input_mode = false;
    let mut cheat_input_buffer = String::new();
    let mut cheat_message: Option<String> = None;
    let mut cheat_message_timer: u32 = 0;
    let mut cheats_submenu = false;
    let mut cheats_selected: usize = 0;
    let mut rewind_buffer = RewindBuffer::new();
    let mut is_rewinding = false;
    let mut thumbnail_cache: [Option<Vec<u8>>; 4] = [None, None, None, None];

    // Netplay state
    let mut netplay = NetplaySession::new();
    let mut netplay_submenu: bool = false;
    let mut netplay_selected: usize = 0;
    let mut netplay_ip_input: String = "127.0.0.1:7777".to_string();
    let mut netplay_ip_editing: bool = false;

    // Lua scripting state
    let mut script_engine: Option<ScriptEngine> = None;
    let mut script_path_arg: Option<String> = None;
    // Parse --script argument
    {
        let args: Vec<String> = env::args().collect();
        let mut i = 1;
        while i < args.len() {
            if args[i] == "--script" {
                if let Some(path) = args.get(i + 1) {
                    script_path_arg = Some(path.clone());
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }
    }

    let mut show_fps = false;
    let mut show_help = false;
    let mut fps_timer = std::time::Instant::now();
    let mut fps_frames: u32 = 0;
    let mut fps_display: String = String::new();

    // Achievement system state
    let mut achievement_engine = AchievementEngine::new();
    let mut achievement_submenu = false;
    let mut controls_submenu = false;

    // Recording & playback state
    let mut recorder = InputRecording::new([0u8; 32]);
    let mut current_rom_name = String::new();
    let mut current_rom_path = String::new();

    // Check command-line argument for direct ROM load
    let args: Vec<String> = env::args().collect();
    if let Some(rom_path) = args.get(1) {
        match fs::read(rom_path) {
            Ok(rom_data) => {
                match Cartridge::new_with_romdb(&rom_data, Some(&romdb)) {
                    Ok(cart) => {
                        let rom_title = cart.rom_title.clone();
                        let mut bus = Bus::new(cart);
                        bus.set_apu_sample_rate(actual_sample_rate);
                        if config.region == "pal" {
                            bus.ppu.set_region(Region::Pal);
                        }
                        let mut cpu = Cpu::new();
                        cpu.reset(&mut bus);
                        add_recent_game(&mut config, rom_path);
                        save_config(&config);
                        auto_load_sram(&mut bus, &config);
                        rewind_buffer.clear();
                        game_bus = Some(bus);
                        game_cpu = Some(cpu);
                        emulator_state = EmulatorState::Game;
                        println!("Loaded: {}", rom_path);
                        let game_name = rom_title.unwrap_or_else(|| {
                            Path::new(rom_path).file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Unknown".to_string())
                        });
                        window_title = format!("OxideNES — {}", game_name);
                        window.set_title(&window_title);
                        // Initialize achievements & recording for this ROM
                        let rom_md5 = md5_hex(&rom_data);
                        achievement_engine = AchievementEngine::load_for_rom(&rom_md5);
                        let rom_sha = sha256(&rom_data);
                        recorder = InputRecording::new(rom_sha);
                        current_rom_name = game_name;
                        current_rom_path = rom_path.clone();
                        // Load persisted Game Genie cheats for this ROM
                        if let Some(ref mut bus) = game_bus {
                            bus.cheats = load_cheats(&current_rom_name);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading ROM: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading file '{}': {}", rom_path, e);
            }
        }
    }

    // Load script from --script arg (after ROM is loaded)
    if let Some(ref spath) = script_path_arg {
        let mut engine = ScriptEngine::init();
        match engine.load_script(spath) {
            Ok(()) => {
                eprintln!("[scripting] Script loaded from --script arg: {}", spath);
                script_engine = Some(engine);
            }
            Err(e) => {
                eprintln!("[scripting] Failed to load script: {}", e);
            }
        }
    }

    // Cached validity for favorites/recents — avoids Path::exists() syscalls per frame
    let mut favorites_valid: Vec<bool> = config.favorite_games.iter()
        .map(|p| std::path::Path::new(p.as_str()).exists())
        .collect();
    let mut recents_valid: Vec<bool> = config.recent_games.iter()
        .map(|p| std::path::Path::new(p.as_str()).exists())
        .collect();

    while window.is_open() {
        let mut next_state: Option<EmulatorState> = None;
        
        if sound_cooldown > 0 { sound_cooldown -= 1; }

        match emulator_state {
            EmulatorState::Menu(ref mut menu) => {
                // Update cursor blink (~500ms at 60fps)
                menu.cursor_timer += 1;
                if menu.cursor_timer >= 30 {
                    menu.cursor_timer = 0;
                    menu.cursor_visible = !menu.cursor_visible;
                }

                let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker, config.input_bindings.controller_p1.deadzone, &mut stick_state_menu);

                let mut action: Option<MenuAction> = None;
                let mut input_back_crt = false;

                match menu.submenu {
                    None => {
                        // Compute item layout: favorites, then recent (non-fav), then browse, settings
                        let valid_favorites: Vec<String> = config.favorite_games.iter().enumerate()
                            .filter(|(i, _)| favorites_valid.get(*i).copied().unwrap_or(false))
                            .map(|(_, p)| p.clone())
                            .collect();
                        let total_favs = valid_favorites.len();
                        let per_page = 5usize;
                        let total_pages = if total_favs == 0 { 0 } else { (total_favs + per_page - 1) / per_page };
                        let page = menu.favorites_page.min(total_pages.saturating_sub(1));
                        let page_start = page * per_page;
                        let page_end = (page_start + per_page).min(total_favs);
                        let fav_count = page_end - page_start;
                        let recent_non_fav: Vec<String> = config.recent_games.iter()
                            .filter(|p| !config.favorite_games.contains(p))
                            .map(|p| p.clone())
                            .collect();
                        let max_recent = 8usize;
                        let recent_count = recent_non_fav.len().min(max_recent);
                        let browse_idx = fav_count + recent_count;
                        let settings_idx = browse_idx + 1;
                        let total_items = settings_idx + 1;

                        // Select cycles favorites page
                        if input.select && total_pages > 1 {
                            menu.favorites_page = (menu.favorites_page + 1) % total_pages;
                            // Reset cursor to first favorite on new page
                            menu.selected = 0;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }

                        if input.up && menu.selected > 0 {
                            menu.selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }
                        if input.down && menu.selected < total_items - 1 {
                            menu.selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }
                        // Toggle favorite with F key / Y button
                        if input.favorite {
                            let game_path = if menu.selected < fav_count {
                                Some(valid_favorites[page_start + menu.selected].clone())
                            } else if menu.selected < fav_count + recent_count {
                                Some(recent_non_fav[menu.selected - fav_count].clone())
                            } else {
                                None
                            };
                            if let Some(path) = game_path {
                                let added = toggle_favorite(&mut config, &path);
                                save_config(&config);
                                favorites_valid = config.favorite_games.iter()
                                    .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                recents_valid = config.recent_games.iter()
                                    .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                if added {
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                } else {
                                    play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                                    // Adjust selection if item was removed from favorites section
                                    if menu.selected < fav_count && menu.selected > 0 {
                                        menu.selected -= 1;
                                    }
                                    // If page now empty, go back a page
                                    let new_total = config.favorite_games.iter()
                                        .filter(|p| std::path::Path::new(p.as_str()).exists())
                                        .count();
                                    let new_pages = if new_total == 0 { 0 } else { (new_total + per_page - 1) / per_page };
                                    if menu.favorites_page >= new_pages && menu.favorites_page > 0 {
                                        menu.favorites_page -= 1;
                                    }
                                }
                            }
                        }
                        if input.confirm {
                            if menu.selected < fav_count {
                                let path = valid_favorites[page_start + menu.selected].clone();
                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                action = Some(MenuAction::LoadRom(path));
                            } else if menu.selected < fav_count + recent_count {
                                let path = recent_non_fav[menu.selected - fav_count].clone();
                                if Path::new(&path).exists() {
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    action = Some(MenuAction::LoadRom(path));
                                } else {
                                    play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                            } else if menu.selected == browse_idx {
                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                menu.submenu = Some(SubMenu::FileBrowser(FileBrowser::new(config.rom_directory.as_deref())));
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                menu.transition_timer = 6;
                                menu.transition_out = false;
                            } else if menu.selected == settings_idx {
                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                menu.submenu = Some(SubMenu::Settings { selected: 0, value_flash: 0 });
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                menu.transition_timer = 6;
                                menu.transition_out = false;
                            }
                        }
                        // Handle update banner: U to open download URL, Esc to dismiss
                        if !update_dismissed {
                            if let Some(info) = updater.get_update() {
                                if window.is_key_pressed(Key::U, KeyRepeat::No) {
                                    let url = if info.download_url.is_empty() {
                                        format!("https://github.com/deaddeadbeef/OxideNES/releases/tag/{}", info.version)
                                    } else {
                                        info.download_url.clone()
                                    };
                                    // Validate URL before opening to prevent command injection
                                    if url.starts_with("https://github.com/") || url.starts_with("https://api.github.com/") {
                                        #[cfg(target_os = "windows")]
                                        {
                                            let _ = std::process::Command::new("cmd")
                                                .args(["/C", "start", "", &url])
                                                .spawn();
                                        }
                                        #[cfg(not(target_os = "windows"))]
                                        {
                                            let _ = std::process::Command::new("xdg-open")
                                                .arg(&url)
                                                .spawn();
                                        }
                                    } else {
                                        eprintln!("Suspicious update URL rejected: {}", url);
                                    }
                                    update_dismissed = true;
                                }
                            }
                        }
                        if input.back {
                            break;
                        }
                    }
                    Some(SubMenu::Settings { ref mut selected, ref mut value_flash }) => {
                        if input.up && *selected > 0 {
                            *selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3; // skip 3 frames between beeps
                            }
                        }
                        if input.down && *selected < 8 {
                            *selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3; // skip 3 frames between beeps
                            }
                        }
                        if input.confirm || input.left || input.right {
                            let is_slider_adjust = (input.left || input.right) && (*selected == 2 || *selected == 3);
                            match *selected {
                                0 => {
                                    crt_enabled = !crt_enabled;
                                    config.crt_enabled = crt_enabled;
                                    save_config(&config);
                                }
                                1 => {
                                    barrel_distortion = !barrel_distortion;
                                    config.barrel_distortion = barrel_distortion;
                                    save_config(&config);
                                }
                                2 => {
                                    if input.right || input.confirm {
                                        if glass_intensity < 100 {
                                            glass_intensity = (glass_intensity + 5).min(100);
                                        }
                                    }
                                    if input.left {
                                        glass_intensity = glass_intensity.saturating_sub(5);
                                    }
                                    config.glass_intensity = glass_intensity;
                                    ca_table = build_ca_table(SCREEN_W, SCREEN_H, glass_intensity);
                                    ghost_alpha_table = build_ghost_alpha_table(glass_intensity);
                                    save_config(&config);
                                    *value_flash = 8;
                                }
                                3 => {
                                    if input.right || input.confirm {
                                        if audio_volume < 100 {
                                            audio_volume = (audio_volume + 5).min(100);
                                        }
                                    }
                                    if input.left {
                                        audio_volume = audio_volume.saturating_sub(5);
                                    }
                                    config.audio_volume = audio_volume;
                                    save_config(&config);
                                    *value_flash = 8;
                                }
                                4 => {
                                    // Toggle region
                                    if config.region == "pal" {
                                        config.region = "ntsc".to_string();
                                    } else {
                                        config.region = "pal".to_string();
                                    }
                                    save_config(&config);
                                }
                                5 => {
                                    // Open CRT settings
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    menu.submenu = Some(SubMenu::CrtSettings { selected: 0, tables_dirty: false, value_flash: 0 });
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                    menu.transition_timer = 6;
                                    menu.transition_out = false;
                                    return; // Skip the confirm sound below
                                }
                                6 => {
                                    // Open input settings
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    menu.submenu = Some(SubMenu::InputSettings(InputSettingsState {
                                        tab: 0,
                                        selected: 0,
                                        waiting_for_input: false,
                                        bindings: config.input_bindings.clone(),
                                        conflict_message: None,
                                        conflict_timer: 0,
                                    }));
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                    menu.transition_timer = 6;
                                    menu.transition_out = false;
                                    return; // Skip the confirm sound below
                                }
                                7 => {
                                    // Toggle check for updates
                                    config.check_for_updates = !config.check_for_updates;
                                    save_config(&config);
                                }
                                8 => {
                                    // Open folder setup to change ROM directory
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    menu.submenu = Some(SubMenu::FolderSetup {
                                        browser: FileBrowser::new(config.rom_directory.as_deref()),
                                        from_settings: true,
                                    });
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                    menu.transition_timer = 6;
                                    menu.transition_out = false;
                                    return;
                                }
                                _ => {}
                            }
                            play_menu_sound(&mut producer, if is_slider_adjust { MenuSound::Adjust } else { MenuSound::Confirm }, actual_sample_rate, audio_volume as f32 / 100.0);
                        }
                        if input.back {
                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            menu.submenu = None;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            menu.transition_timer = 6;
                            menu.transition_out = false;
                        }
                    }
                    Some(SubMenu::CrtSettings { ref mut selected, ref mut tables_dirty, ref mut value_flash }) => {
                        if input.up && *selected > 0 {
                            *selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }
                        if input.down && *selected < 8 {
                            *selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }
                        if input.left || input.right {
                            let delta: i16 = if input.right { 5 } else { -5 };
                            match *selected {
                                0 => {
                                    config.crt_config.scanline_intensity = (config.crt_config.scanline_intensity as i16 + delta).clamp(0, 100) as u8;
                                    *tables_dirty = true;
                                }
                                1 => {
                                    config.crt_config.phosphor_warmth = (config.crt_config.phosphor_warmth as i16 + delta).clamp(0, 100) as u8;
                                    *tables_dirty = true;
                                }
                                2 => {
                                    config.crt_config.vignette_strength = (config.crt_config.vignette_strength as i16 + delta).clamp(0, 100) as u8;
                                    *tables_dirty = true;
                                }
                                3 => {
                                    config.crt_config.blur_amount = (config.crt_config.blur_amount as i16 + delta).clamp(0, 100) as u8;
                                    *tables_dirty = true;
                                }
                                4 => {
                                    config.crt_config.curvature_strength = (config.crt_config.curvature_strength as i16 + delta).clamp(0, 100) as u8;
                                    *tables_dirty = true;
                                }
                                5 => {
                                    // Glass intensity (existing field)
                                    glass_intensity = (glass_intensity as i16 + delta).clamp(0, 100) as u8;
                                    config.glass_intensity = glass_intensity;
                                    ca_table = build_ca_table(SCREEN_W, SCREEN_H, glass_intensity);
                                    ghost_alpha_table = build_ghost_alpha_table(glass_intensity);
                                }
                                6 => {
                                    // Cycle mask mode
                                    config.crt_config.mask_mode = match config.crt_config.mask_mode {
                                        CrtMaskMode::Off => if input.right { CrtMaskMode::ShadowMask } else { CrtMaskMode::SlotMask },
                                        CrtMaskMode::ShadowMask => if input.right { CrtMaskMode::ApertureGrille } else { CrtMaskMode::Off },
                                        CrtMaskMode::ApertureGrille => if input.right { CrtMaskMode::SlotMask } else { CrtMaskMode::ShadowMask },
                                        CrtMaskMode::SlotMask => if input.right { CrtMaskMode::Off } else { CrtMaskMode::ApertureGrille },
                                    };
                                    mask_table = build_mask_table(&config.crt_config.mask_mode);
                                    *tables_dirty = true;
                                }
                                7 => {
                                    config.crt_config.mask_intensity = (config.crt_config.mask_intensity as i16 + delta).clamp(0, 100) as u8;
                                    *tables_dirty = true;
                                }
                                _ => {}
                            }
                            play_menu_sound(&mut producer, MenuSound::Adjust, actual_sample_rate, audio_volume as f32 / 100.0);
                            save_config(&config);
                            *value_flash = 8;
                        }
                        if input.confirm && *selected == 8 {
                            // BACK
                            input_back_crt = true;
                        }
                        if input.back || input_back_crt {
                            if *tables_dirty {
                                vignette_table = build_vignette_table_with_strength(config.crt_config.vignette_strength);
                                distortion_table = build_distortion_table_with_curvature(config.crt_config.curvature_strength);
                            }
                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            menu.submenu = Some(SubMenu::Settings { selected: 5, value_flash: 0 });
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            menu.transition_timer = 6;
                            menu.transition_out = false;
                        }
                    }
                    Some(SubMenu::FileBrowser(ref mut browser)) => {
                        // Update error timer
                        if browser.error_timer > 0 {
                            browser.error_timer -= 1;
                            if browser.error_timer == 0 {
                                browser.error_message = None;
                            }
                        }

                        let count = browser.entries.len();
                        if count > 0 {
                            if input.up && browser.selected > 0 {
                                browser.selected -= 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                if browser.selected < browser.scroll_offset {
                                    browser.scroll_offset = browser.selected;
                                }
                            }
                            if input.down && browser.selected < count - 1 {
                                browser.selected += 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3; // skip 3 frames between beeps
                                }
                                if browser.selected >= browser.scroll_offset + 20 {
                                    browser.scroll_offset = browser.selected - 19;
                                }
                            }
                            if (input.page_up || input.left) && browser.selected > 0 {
                                browser.selected = browser.selected.saturating_sub(10);
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3; // skip 3 frames between beeps
                                }
                                if browser.selected < browser.scroll_offset {
                                    browser.scroll_offset = browser.selected;
                                }
                            }
                            if (input.page_down || input.right) && count > 0 {
                                browser.selected = (browser.selected + 10).min(count - 1);
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3; // skip 3 frames between beeps
                                }
                                if browser.selected >= browser.scroll_offset + 20 {
                                    browser.scroll_offset = browser.selected.saturating_sub(19);
                                }
                            }
                            if input.confirm {
                                let entry_is_dir = browser.entries[browser.selected].is_dir;
                                let entry_path = browser.entries[browser.selected].full_path.clone();
                                if entry_is_dir {
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    browser.navigate_to(&entry_path);
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                } else {
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    action = Some(MenuAction::LoadRom(
                                        entry_path.to_string_lossy().to_string(),
                                    ));
                                }
                            }
                            if input.favorite {
                                if let Some(entry) = browser.entries.get(browser.selected) {
                                    if !entry.is_dir {
                                        let path_str = entry.full_path.to_string_lossy().to_string();
                                        let added = toggle_favorite(&mut config, &path_str);
                                        save_config(&config);
                                        favorites_valid = config.favorite_games.iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                        recents_valid = config.recent_games.iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                        if added {
                                            play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                        } else {
                                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                                        }
                                    }
                                }
                            }
                        }
                        if input.back || input.backspace {
                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            let parent = browser.current_dir.parent().map(|p| p.to_path_buf());
                            if let Some(parent) = parent {
                                if !parent.as_os_str().is_empty() {
                                    browser.navigate_to(&parent);
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                } else {
                                    menu.submenu = None;
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                    menu.transition_timer = 6;
                                    menu.transition_out = false;
                                }
                            } else {
                                menu.submenu = None;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                menu.transition_timer = 6;
                                menu.transition_out = false;
                            }
                        }
                    }
                    Some(SubMenu::InputSettings(ref mut state)) => {
                        // Handle conflict timer
                        if state.conflict_timer > 0 {
                            state.conflict_timer -= 1;
                            if state.conflict_timer == 0 {
                                state.conflict_message = None;
                            }
                        }

                        if state.waiting_for_input {
                            // In key capture mode - handle raw input differently
                            let mut captured = false;
                            
                            if state.tab < 2 {
                                // Keyboard capture
                                let keys = window.get_keys_pressed(KeyRepeat::No);
                                if let Some(&key) = keys.first() {
                                    if key != Key::Escape {
                                        let key_string = key_to_string(key);
                                        
                                        // Check for conflicts within the same player
                                        let binding_refs = if state.tab == 0 {
                                            [&state.bindings.keyboard_p1.up, &state.bindings.keyboard_p1.down, &state.bindings.keyboard_p1.left, &state.bindings.keyboard_p1.right,
                                             &state.bindings.keyboard_p1.a, &state.bindings.keyboard_p1.b, &state.bindings.keyboard_p1.start, &state.bindings.keyboard_p1.select,
                                             &state.bindings.keyboard_p1.turbo_a, &state.bindings.keyboard_p1.turbo_b]
                                        } else {
                                            [&state.bindings.keyboard_p2.up, &state.bindings.keyboard_p2.down, &state.bindings.keyboard_p2.left, &state.bindings.keyboard_p2.right,
                                             &state.bindings.keyboard_p2.a, &state.bindings.keyboard_p2.b, &state.bindings.keyboard_p2.start, &state.bindings.keyboard_p2.select,
                                             &state.bindings.keyboard_p2.turbo_a, &state.bindings.keyboard_p2.turbo_b]
                                        };
                                        
                                        let binding_names = ["UP", "DOWN", "LEFT", "RIGHT", "A", "B", "START", "SELECT", "TURBO A", "TURBO B"];
                                        let old_value = binding_refs[state.selected].clone();
                                        let mut conflict_idx: Option<usize> = None;
                                        
                                        for (i, &existing_key) in binding_refs.iter().enumerate() {
                                            if i != state.selected && existing_key == &key_string {
                                                state.conflict_message = Some(format!("Swapped with {}", binding_names[i]));
                                                state.conflict_timer = 90;
                                                conflict_idx = Some(i);
                                                break;
                                            }
                                        }
                                        
                                        // Apply the binding
                                        match state.tab {
                                            0 => match state.selected {
                                                0 => state.bindings.keyboard_p1.up = key_string,
                                                1 => state.bindings.keyboard_p1.down = key_string,
                                                2 => state.bindings.keyboard_p1.left = key_string,
                                                3 => state.bindings.keyboard_p1.right = key_string,
                                                4 => state.bindings.keyboard_p1.a = key_string,
                                                5 => state.bindings.keyboard_p1.b = key_string,
                                                6 => state.bindings.keyboard_p1.start = key_string,
                                                7 => state.bindings.keyboard_p1.select = key_string,
                                                8 => state.bindings.keyboard_p1.turbo_a = key_string,
                                                9 => state.bindings.keyboard_p1.turbo_b = key_string,
                                                _ => {}
                                            },
                                            1 => match state.selected {
                                                0 => state.bindings.keyboard_p2.up = key_string,
                                                1 => state.bindings.keyboard_p2.down = key_string,
                                                2 => state.bindings.keyboard_p2.left = key_string,
                                                3 => state.bindings.keyboard_p2.right = key_string,
                                                4 => state.bindings.keyboard_p2.a = key_string,
                                                5 => state.bindings.keyboard_p2.b = key_string,
                                                6 => state.bindings.keyboard_p2.start = key_string,
                                                7 => state.bindings.keyboard_p2.select = key_string,
                                                8 => state.bindings.keyboard_p2.turbo_a = key_string,
                                                9 => state.bindings.keyboard_p2.turbo_b = key_string,
                                                _ => {}
                                            },
                                            _ => {}
                                        }
                                        
                                        // Swap: set conflicting binding to the old value
                                        if let Some(ci) = conflict_idx {
                                            match state.tab {
                                                0 => match ci {
                                                    0 => state.bindings.keyboard_p1.up = old_value,
                                                    1 => state.bindings.keyboard_p1.down = old_value,
                                                    2 => state.bindings.keyboard_p1.left = old_value,
                                                    3 => state.bindings.keyboard_p1.right = old_value,
                                                    4 => state.bindings.keyboard_p1.a = old_value,
                                                    5 => state.bindings.keyboard_p1.b = old_value,
                                                    6 => state.bindings.keyboard_p1.start = old_value,
                                                    7 => state.bindings.keyboard_p1.select = old_value,
                                                    8 => state.bindings.keyboard_p1.turbo_a = old_value,
                                                    9 => state.bindings.keyboard_p1.turbo_b = old_value,
                                                    _ => {}
                                                },
                                                1 => match ci {
                                                    0 => state.bindings.keyboard_p2.up = old_value,
                                                    1 => state.bindings.keyboard_p2.down = old_value,
                                                    2 => state.bindings.keyboard_p2.left = old_value,
                                                    3 => state.bindings.keyboard_p2.right = old_value,
                                                    4 => state.bindings.keyboard_p2.a = old_value,
                                                    5 => state.bindings.keyboard_p2.b = old_value,
                                                    6 => state.bindings.keyboard_p2.start = old_value,
                                                    7 => state.bindings.keyboard_p2.select = old_value,
                                                    8 => state.bindings.keyboard_p2.turbo_a = old_value,
                                                    9 => state.bindings.keyboard_p2.turbo_b = old_value,
                                                    _ => {}
                                                },
                                                _ => {}
                                            }
                                        }
                                        
                                        captured = true;
                                    }
                                }
                            } else {
                                // Controller capture
                                if let Some(ref mut g) = gilrs {
                                    while let Some(event) = g.next_event() {
                                        if let gilrs::EventType::ButtonPressed(btn, _) = event.event {
                                            let button_string = gilrs_button_to_string(btn);
                                            
                                            // Check for conflicts within the same controller
                                            let binding_refs = if state.tab == 2 {
                                                [&state.bindings.controller_p1.a, &state.bindings.controller_p1.b,
                                                 &state.bindings.controller_p1.turbo_a, &state.bindings.controller_p1.turbo_b,
                                                 &state.bindings.controller_p1.start, &state.bindings.controller_p1.select]
                                            } else {
                                                [&state.bindings.controller_p2.a, &state.bindings.controller_p2.b,
                                                 &state.bindings.controller_p2.turbo_a, &state.bindings.controller_p2.turbo_b,
                                                 &state.bindings.controller_p2.start, &state.bindings.controller_p2.select]
                                            };
                                            
                                            let binding_names = ["A", "B", "TURBO A", "TURBO B", "START", "SELECT"];
                                            let old_value = binding_refs[state.selected].clone();
                                            let mut conflict_idx: Option<usize> = None;
                                            
                                            for (i, &existing) in binding_refs.iter().enumerate() {
                                                if i != state.selected && existing == &button_string {
                                                    state.conflict_message = Some(format!("Swapped with {}", binding_names[i]));
                                                    state.conflict_timer = 90;
                                                    conflict_idx = Some(i);
                                                    break;
                                                }
                                            }
                                            
                                            // Apply the binding
                                            match state.tab {
                                                2 => match state.selected {
                                                    0 => state.bindings.controller_p1.a = button_string,
                                                    1 => state.bindings.controller_p1.b = button_string,
                                                    2 => state.bindings.controller_p1.turbo_a = button_string,
                                                    3 => state.bindings.controller_p1.turbo_b = button_string,
                                                    4 => state.bindings.controller_p1.start = button_string,
                                                    5 => state.bindings.controller_p1.select = button_string,
                                                    _ => {}
                                                },
                                                3 => match state.selected {
                                                    0 => state.bindings.controller_p2.a = button_string,
                                                    1 => state.bindings.controller_p2.b = button_string,
                                                    2 => state.bindings.controller_p2.turbo_a = button_string,
                                                    3 => state.bindings.controller_p2.turbo_b = button_string,
                                                    4 => state.bindings.controller_p2.start = button_string,
                                                    5 => state.bindings.controller_p2.select = button_string,
                                                    _ => {}
                                                },
                                                _ => {}
                                            }
                                            
                                            // Swap: set conflicting binding to the old value
                                            if let Some(ci) = conflict_idx {
                                                match state.tab {
                                                    2 => match ci {
                                                        0 => state.bindings.controller_p1.a = old_value,
                                                        1 => state.bindings.controller_p1.b = old_value,
                                                        2 => state.bindings.controller_p1.turbo_a = old_value,
                                                        3 => state.bindings.controller_p1.turbo_b = old_value,
                                                        4 => state.bindings.controller_p1.start = old_value,
                                                        5 => state.bindings.controller_p1.select = old_value,
                                                        _ => {}
                                                    },
                                                    3 => match ci {
                                                        0 => state.bindings.controller_p2.a = old_value,
                                                        1 => state.bindings.controller_p2.b = old_value,
                                                        2 => state.bindings.controller_p2.turbo_a = old_value,
                                                        3 => state.bindings.controller_p2.turbo_b = old_value,
                                                        4 => state.bindings.controller_p2.start = old_value,
                                                        5 => state.bindings.controller_p2.select = old_value,
                                                        _ => {}
                                                    },
                                                    _ => {}
                                                }
                                            }
                                            
                                            captured = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            
                            if captured {
                                state.waiting_for_input = false;
                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                            }
                            
                            if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                                // Cancel capture
                                state.waiting_for_input = false;
                                play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            }
                        } else {
                            // Normal navigation mode
                            let max_items = if state.tab < 2 { 9 } else { 6 }; // 10 keyboard items (0-9), 7 controller items (0-6, including deadzone)
                            
                            if input.up && state.selected > 0 {
                                state.selected -= 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            if input.down && state.selected < max_items {
                                state.selected += 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            
                            // Tab switching (skip when deadzone row is active on controller tabs)
                            let deadzone_active = state.tab >= 2 && state.selected == 6;
                            if input.left && state.tab > 0 && !deadzone_active {
                                state.tab -= 1;
                                state.selected = 0;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            if input.right && state.tab < 3 && !deadzone_active {
                                state.tab += 1;
                                state.selected = 0;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            
                            // Handle Tab key for switching tabs
                            if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
                                state.tab = (state.tab + 1) % 4;
                                state.selected = 0;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            
                            if input.confirm {
                                // Start rebinding process
                                if state.tab >= 2 && state.selected == 6 {
                                    // Special case for deadzone - adjust with left/right instead of rebinding
                                } else {
                                    state.waiting_for_input = true;
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                            }
                            
                            // Handle deadzone adjustment for controller tabs
                            if state.tab >= 2 && state.selected == 6 && (input.left || input.right) {
                                let deadzone = if state.tab == 2 { &mut state.bindings.controller_p1.deadzone } else { &mut state.bindings.controller_p2.deadzone };
                                if input.left {
                                    *deadzone = (*deadzone - 0.05).max(0.10);
                                }
                                if input.right {
                                    *deadzone = (*deadzone + 0.05).min(0.80);
                                }
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            
                            if input.back {
                                // Save and go back
                                config.input_bindings = state.bindings.clone();
                                save_config(&config);
                                play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                                menu.submenu = Some(SubMenu::Settings { selected: 4, value_flash: 0 }); // Return to settings, INPUT SETTINGS selected
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                menu.transition_timer = 6;
                                menu.transition_out = false;
                            }
                        }
                    }
                    Some(SubMenu::FolderSetup { ref mut browser, from_settings }) => {
                        // Update error timer
                        if browser.error_timer > 0 {
                            browser.error_timer -= 1;
                            if browser.error_timer == 0 {
                                browser.error_message = None;
                            }
                        }

                        let count = browser.entries.len();
                        if count > 0 {
                            if input.up && browser.selected > 0 {
                                browser.selected -= 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                                if browser.selected < browser.scroll_offset {
                                    browser.scroll_offset = browser.selected;
                                }
                            }
                            if input.down && browser.selected < count - 1 {
                                browser.selected += 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                                if browser.selected >= browser.scroll_offset + 14 {
                                    browser.scroll_offset = browser.selected.saturating_sub(13);
                                }
                            }
                            if input.confirm {
                                let entry_is_dir = browser.entries[browser.selected].is_dir;
                                let entry_path = browser.entries[browser.selected].full_path.clone();
                                if entry_is_dir {
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    browser.navigate_to(&entry_path);
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                }
                                // Files are not selectable in folder setup mode
                            }
                        }

                        // Determine transition action (extract data before releasing borrow)
                        let mut folder_action: u8 = 0; // 0=none, 1=select folder, 2=back to settings
                        let mut selected_dir = String::new();

                        if input.select {
                            selected_dir = browser.current_dir.to_string_lossy().to_string();
                            folder_action = 1;
                        } else if input.back || input.backspace {
                            let parent = browser.current_dir.parent().map(|p| p.to_path_buf());
                            if let Some(parent) = parent {
                                if !parent.as_os_str().is_empty() {
                                    play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                                    browser.navigate_to(&parent);
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                } else if from_settings {
                                    folder_action = 2;
                                }
                            } else if from_settings {
                                folder_action = 2;
                            }
                        }

                        // Apply transitions (borrow released by match arm end via early extraction)
                        let fs_from_settings = from_settings;
                        if folder_action == 1 {
                            config.rom_directory = Some(selected_dir);
                            save_config(&config);
                            play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                            if fs_from_settings {
                                menu.submenu = Some(SubMenu::Settings { selected: 8, value_flash: 0 });
                            } else {
                                menu.submenu = None;
                            }
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            menu.transition_timer = 6;
                            menu.transition_out = false;
                        } else if folder_action == 2 {
                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            menu.submenu = Some(SubMenu::Settings { selected: 8, value_flash: 0 });
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            menu.transition_timer = 6;
                            menu.transition_out = false;
                        }
                    }
                }

                // Process actions
                match action {
                    Some(MenuAction::LoadRom(path_str)) => {
                        match fs::read(&path_str) {
                            Ok(rom_data) => {
                                // Show loading indicator
                                draw_text_centered_8x8(&mut menu_framebuffer, "LOADING...", 15, 0xF8D878);
                                let dt = if barrel_distortion { &distortion_table } else { &flat_distortion_table };
                                if crt_enabled {
                                    crt_filter(&menu_framebuffer, &mut crt_buffer, &vignette_table, dt, &config.crt_config, &mask_table);
                                    // Phosphor bloom — bright pixels glow into neighbors
                                    apply_phosphor_bloom(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                                    apply_scanline_glow(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                                } else {
                                    scale_simple(&menu_framebuffer, &mut crt_buffer);
                                }
                                composite_screen_fast(&mut composite_buffer, &crt_buffer, WINDOW_WIDTH);
                                let _ = window.update_with_buffer(&composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT);

                                match Cartridge::new_with_romdb(&rom_data, Some(&romdb)) {
                                    Ok(cart) => {
                                        let rom_title = cart.rom_title.clone();
                                        let mut bus = Bus::new(cart);
                                        bus.set_apu_sample_rate(actual_sample_rate);
                                        if config.region == "pal" {
                                            bus.ppu.set_region(Region::Pal);
                                        }
                                        let mut cpu = Cpu::new();
                                        cpu.reset(&mut bus);
                                        add_recent_game(&mut config, &path_str);
                                        save_config(&config);
                                        favorites_valid = config.favorite_games.iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                        recents_valid = config.recent_games.iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                        auto_load_sram(&mut bus, &config);
                                        rewind_buffer.clear();
                                        game_bus = Some(bus);
                                        game_cpu = Some(cpu);
                                        next_state = Some(EmulatorState::Game);
                                        println!("Loaded: {}", path_str);
                                        let game_name = rom_title.unwrap_or_else(|| {
                                            Path::new(&path_str).file_stem()
                                                .map(|s| s.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "Unknown".to_string())
                                        });
                                        window_title = format!("OxideNES — {}", game_name);
                                        window.set_title(&window_title);
                                        // Initialize achievements & recording for this ROM
                                        let rom_md5 = md5_hex(&rom_data);
                                        achievement_engine = AchievementEngine::load_for_rom(&rom_md5);
                                        let rom_sha = sha256(&rom_data);
                                        recorder = InputRecording::new(rom_sha);
                                        current_rom_name = game_name;
                                        current_rom_path = path_str.clone();
                                        // Load persisted Game Genie cheats for this ROM
                                        if let Some(ref mut bus) = game_bus {
                                            bus.cheats = load_cheats(&current_rom_name);
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("{}", e);
                                        if let Some(SubMenu::FileBrowser(ref mut browser)) = menu.submenu {
                                            browser.error_message = Some(msg.clone());
                                            browser.error_timer = 180;
                                        }
                                        eprintln!("ROM Error: {}", msg);
                                    }
                                }
                            }
                            Err(e) => {
                                play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                let msg = format!("READ ERROR: {}", e);
                                if let Some(SubMenu::FileBrowser(ref mut browser)) = menu.submenu {
                                    browser.error_message = Some(msg.clone());
                                    browser.error_timer = 180;
                                }
                                eprintln!("{}", msg);
                            }
                        }
                    }
                    None => {}
                }

                // Apply state transition immediately (don't fall through to menu render)
                if let Some(new_state) = next_state.take() {
                    emulator_state = new_state;
                    continue;
                }

                // Render menu to 256x240 framebuffer
                match menu.submenu {
                    None => {
                        render_home_screen(&mut menu_framebuffer, menu, &config, menu.cursor_visible, &favorites_valid, &recents_valid);
                        // Show update banner if available and not dismissed
                        if !update_dismissed {
                            if let Some(info) = updater.get_update() {
                                let banner = format!("UPDATE: {}", info.version);
                                draw_text_centered_8x8(&mut menu_framebuffer, &banner, 28, MENU_GOLD);
                                draw_text_centered_8x8(&mut menu_framebuffer, "U:DOWNLOAD  ESC:DISMISS", 29, MENU_DARK_GRAY);
                            }
                        }
                    }
                    Some(SubMenu::Settings { selected, ref mut value_flash }) => {
                        render_settings(&mut menu_framebuffer, &config, selected, menu.cursor_visible, audio_volume, glass_intensity, *value_flash);
                        if *value_flash > 0 { *value_flash -= 1; }
                    }
                    Some(SubMenu::FileBrowser(ref browser)) => {
                        render_file_browser(&mut menu_framebuffer, browser, menu.cursor_visible, &config);
                    }
                    Some(SubMenu::InputSettings(ref state)) => {
                        render_input_settings(&mut menu_framebuffer, state, menu.cursor_visible);
                    }
                    Some(SubMenu::CrtSettings { selected, ref mut value_flash, .. }) => {
                        render_crt_settings(&mut menu_framebuffer, &config, selected, menu.cursor_visible, *value_flash);
                        if *value_flash > 0 { *value_flash -= 1; }
                    }
                    Some(SubMenu::FolderSetup { ref browser, .. }) => {
                        render_folder_setup(&mut menu_framebuffer, browser, menu.cursor_visible);
                    }
                }

                // Apply screen transition fade
                if menu.transition_timer > 0 {
                    apply_menu_fade(&mut menu_framebuffer, 256, 240, menu.transition_timer);
                    menu.transition_timer -= 1;
                }

                // Apply CRT filter pipeline (same as game!)
                let dt = if barrel_distortion { &distortion_table } else { &flat_distortion_table };
                if crt_enabled {
                    crt_filter(&menu_framebuffer, &mut crt_buffer, &vignette_table, dt, &config.crt_config, &mask_table);
                    // Phosphor bloom — bright pixels glow into neighbors
                    apply_phosphor_bloom(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                    apply_scanline_glow(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                    // Apply chromatic aberration to crt_buffer (screen area only)
                    if glass_intensity > 30 {
                        ca_temp.copy_from_slice(&crt_buffer[..SCREEN_W * SCREEN_H]);
                        apply_chromatic_aberration(&mut crt_buffer, &ca_temp, &ca_table, SCREEN_W, SCREEN_H);
                    }
                } else {
                    scale_simple(&menu_framebuffer, &mut crt_buffer);
                }
                composite_screen_fast(&mut composite_buffer, &crt_buffer, WINDOW_WIDTH);
                if crt_enabled {
                    apply_screen_glare(&mut composite_buffer, &glare_table, &glass_thickness_table, WINDOW_WIDTH, glass_intensity);
                    // Internal ghost reflection from thick CRT glass
                    if glass_intensity > 20 {
                        for y in 0..SCREEN_H {
                            let row_start = (y + SCREEN_Y) * WINDOW_WIDTH + SCREEN_X;
                            ghost_buffer[row_start..row_start + SCREEN_W]
                                .copy_from_slice(&composite_buffer[row_start..row_start + SCREEN_W]);
                        }
                        apply_internal_ghost(&mut composite_buffer, &ghost_buffer, &ghost_alpha_table, WINDOW_WIDTH);
                    }
                }

                window
                    .update_with_buffer(&composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
                    .expect("Failed to update window");
            }

            EmulatorState::Game => {
                if let (Some(ref mut bus), Some(ref mut cpu)) = (&mut game_bus, &mut game_cpu) {
                    // Handle pause menu input
                    if paused && cheats_submenu {
                        // Cheats submenu input handling
                        if cheat_input_mode {
                            // Text input for new Game Genie code
                            for key in &window.get_keys_pressed(KeyRepeat::Yes) {
                                match key {
                                    Key::A => cheat_input_buffer.push('A'),
                                    Key::E => cheat_input_buffer.push('E'),
                                    Key::G => cheat_input_buffer.push('G'),
                                    Key::I => cheat_input_buffer.push('I'),
                                    Key::K => cheat_input_buffer.push('K'),
                                    Key::L => cheat_input_buffer.push('L'),
                                    Key::N => cheat_input_buffer.push('N'),
                                    Key::O => cheat_input_buffer.push('O'),
                                    Key::P => cheat_input_buffer.push('P'),
                                    Key::S => cheat_input_buffer.push('S'),
                                    Key::T => cheat_input_buffer.push('T'),
                                    Key::U => cheat_input_buffer.push('U'),
                                    Key::V => cheat_input_buffer.push('V'),
                                    Key::X => cheat_input_buffer.push('X'),
                                    Key::Y => cheat_input_buffer.push('Y'),
                                    Key::Z => cheat_input_buffer.push('Z'),
                                    Key::Backspace => { cheat_input_buffer.pop(); }
                                    Key::Enter => {
                                        if cheat_input_buffer.len() == 6 || cheat_input_buffer.len() == 8 {
                                            if let Some(code) = oxidenes::bus::GameGenieCode::decode(&cheat_input_buffer) {
                                                bus.cheats.push(code);
                                                save_cheats(&current_rom_name, &bus.cheats);
                                                cheat_message = Some(format!("ADDED: {}", cheat_input_buffer));
                                                cheat_message_timer = 120;
                                                cheat_input_mode = false;
                                            } else {
                                                cheat_message = Some("INVALID CODE".to_string());
                                                cheat_message_timer = 90;
                                            }
                                        } else {
                                            cheat_message = Some("NEED 6 OR 8 CHARS".to_string());
                                            cheat_message_timer = 90;
                                        }
                                        cheat_input_buffer.clear();
                                    }
                                    Key::Escape => {
                                        cheat_input_mode = false;
                                        cheat_input_buffer.clear();
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker, config.input_bindings.controller_p1.deadzone, &mut stick_state_menu);
                            // Items: each cheat (toggle), then ADD CODE, CLEAR ALL
                            let item_count = bus.cheats.len() + 2;
                            if input.up && cheats_selected > 0 {
                                cheats_selected -= 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            if input.down && cheats_selected < item_count - 1 {
                                cheats_selected += 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            if input.confirm {
                                if cheats_selected < bus.cheats.len() {
                                    // Toggle cheat on/off
                                    bus.cheats[cheats_selected].enabled = !bus.cheats[cheats_selected].enabled;
                                    save_cheats(&current_rom_name, &bus.cheats);
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                } else if cheats_selected == bus.cheats.len() {
                                    // ADD CODE
                                    cheat_input_mode = true;
                                    cheat_input_buffer.clear();
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                } else {
                                    // CLEAR ALL
                                    bus.cheats.clear();
                                    save_cheats(&current_rom_name, &bus.cheats);
                                    cheats_selected = 0;
                                    cheat_message = Some("ALL CHEATS CLEARED".to_string());
                                    cheat_message_timer = 90;
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                            }
                            if input.backspace && cheats_selected < bus.cheats.len() {
                                // Delete individual cheat with Backspace
                                bus.cheats.remove(cheats_selected);
                                save_cheats(&current_rom_name, &bus.cheats);
                                if cheats_selected >= bus.cheats.len() + 2 {
                                    cheats_selected = (bus.cheats.len() + 1).max(0);
                                }
                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                            }
                            if input.back {
                                cheats_submenu = false;
                                cheat_input_mode = false;
                                play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            }
                        }
                    } else if paused && controls_submenu {
                        // Controls reference page input handling
                        let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker, config.input_bindings.controller_p1.deadzone, &mut stick_state_menu);
                        if input.back {
                            controls_submenu = false;
                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                        }
                    } else if paused && achievement_submenu {
                        // Achievement submenu input handling
                        let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker, config.input_bindings.controller_p1.deadzone, &mut stick_state_menu);
                        if input.back {
                            achievement_submenu = false;
                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                        }
                    } else if paused && netplay_submenu {
                        // Netplay submenu input handling
                        if netplay_ip_editing {
                            // IP address text input mode
                            for key in &window.get_keys_pressed(KeyRepeat::Yes) {
                                match key {
                                    Key::Key0 | Key::NumPad0 => netplay_ip_input.push('0'),
                                    Key::Key1 | Key::NumPad1 => netplay_ip_input.push('1'),
                                    Key::Key2 | Key::NumPad2 => netplay_ip_input.push('2'),
                                    Key::Key3 | Key::NumPad3 => netplay_ip_input.push('3'),
                                    Key::Key4 | Key::NumPad4 => netplay_ip_input.push('4'),
                                    Key::Key5 | Key::NumPad5 => netplay_ip_input.push('5'),
                                    Key::Key6 | Key::NumPad6 => netplay_ip_input.push('6'),
                                    Key::Key7 | Key::NumPad7 => netplay_ip_input.push('7'),
                                    Key::Key8 | Key::NumPad8 => netplay_ip_input.push('8'),
                                    Key::Key9 | Key::NumPad9 => netplay_ip_input.push('9'),
                                    Key::Period | Key::NumPadDot => netplay_ip_input.push('.'),
                                    Key::Semicolon => netplay_ip_input.push(':'), // shift+; = : on most layouts
                                    Key::Backspace => { netplay_ip_input.pop(); }
                                    Key::Enter => {
                                        match netplay.join(&netplay_ip_input) {
                                            Ok(()) => {
                                                overlay_message = Some("CONNECTING...".to_string());
                                                overlay_timer = 120;
                                                netplay_submenu = false;
                                                paused = false;
                                            }
                                            Err(e) => {
                                                overlay_message = Some(format!("FAILED: {}", e));
                                                overlay_timer = 120;
                                            }
                                        }
                                        netplay_ip_editing = false;
                                    }
                                    Key::Escape => {
                                        netplay_ip_editing = false;
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker, config.input_bindings.controller_p1.deadzone, &mut stick_state_menu);
                            if input.up && netplay_selected > 0 {
                                netplay_selected -= 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            if input.down && netplay_selected < 3 {
                                netplay_selected += 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            if input.confirm {
                                match netplay_selected {
                                    0 => { // Host
                                        match netplay.host(7777) {
                                            Ok(()) => {
                                                overlay_message = Some("HOSTING ON PORT 7777".to_string());
                                                overlay_timer = 120;
                                                netplay_submenu = false;
                                                paused = false;
                                            }
                                            Err(e) => {
                                                overlay_message = Some(format!("HOST FAILED: {}", e));
                                                overlay_timer = 120;
                                            }
                                        }
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    1 => { // Join
                                        netplay_ip_editing = true;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    2 => { // Disconnect
                                        netplay.disconnect();
                                        overlay_message = Some("NETPLAY DISCONNECTED".to_string());
                                        overlay_timer = 90;
                                        netplay_submenu = false;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    3 => { // Input delay toggle (cycle 1-5)
                                        netplay.input_delay = if netplay.input_delay >= 5 { 1 } else { netplay.input_delay + 1 };
                                        play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    _ => {}
                                }
                            }
                            if input.back {
                                netplay_submenu = false;
                                play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            }
                        }
                    } else if paused {
                        let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker, config.input_bindings.controller_p1.deadzone, &mut stick_state_menu);
                        if input.up && pause_selected > 0 {
                            pause_selected -= 1;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }
                        if input.down && pause_selected < 13 {
                            pause_selected += 1;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }
                        // L/R cycles save slot when on Save or Load items
                        if pause_selected == 1 || pause_selected == 2 {
                            if input.left {
                                current_save_slot = if current_save_slot == 1 { 5 } else { current_save_slot - 1 };
                                pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                                pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            if input.right {
                                current_save_slot = if current_save_slot == 5 { 1 } else { current_save_slot + 1 };
                                pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                                pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                        }
                        if input.confirm {
                            match pause_selected {
                                0 => { // Resume
                                    paused = false;
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                                1 => { // Save state
                                    if save_state(bus, cpu, &config, current_save_slot) {
                                        thumbnail_cache[(current_save_slot as usize).saturating_sub(1).min(3)] = load_thumbnail(&config, current_save_slot);
                                        overlay_message = Some("STATE SAVED".to_string());
                                        overlay_timer = 90;
                                        paused = false;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    } else {
                                        overlay_message = Some("NO SRAM FOUND".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                }
                                2 => { // Load state
                                    if load_state(bus, cpu, &config, current_save_slot) {
                                        overlay_message = Some("STATE LOADED".to_string());
                                        overlay_timer = 90;
                                        paused = false;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    } else {
                                        overlay_message = Some("NO SAVE FOUND".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                }
                                3 => { // Cheats submenu
                                    cheats_submenu = true;
                                    cheats_selected = 0;
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                                4 => { // Netplay
                                    netplay_submenu = true;
                                    netplay_selected = 0;
                                    netplay_ip_editing = false;
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                                5 => { // Reload / Load script
                                    let path = script_engine.as_ref()
                                        .and_then(|s| s.script_path.clone())
                                        .or_else(|| script_path_arg.clone());
                                    if let Some(spath) = path {
                                        let mut engine = ScriptEngine::init();
                                        match engine.load_script(&spath) {
                                            Ok(()) => {
                                                script_engine = Some(engine);
                                                overlay_message = Some("SCRIPT LOADED".to_string());
                                                overlay_timer = 90;
                                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                            }
                                            Err(e) => {
                                                eprintln!("[scripting] {}", e);
                                                overlay_message = Some("SCRIPT ERROR".to_string());
                                                overlay_timer = 90;
                                                play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                            }
                                        }
                                    } else {
                                        overlay_message = Some("NO SCRIPT SET (--script)".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    paused = false;
                                }
                                6 => { // Unload script
                                    if let Some(ref mut engine) = script_engine {
                                        engine.unload();
                                        overlay_message = Some("SCRIPT UNLOADED".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    script_engine = None;
                                    paused = false;
                                }
                                7 => { // Toggle favorite
                                    if !current_rom_path.is_empty() {
                                        let added = toggle_favorite(&mut config, &current_rom_path);
                                        save_config(&config);
                                        favorites_valid = config.favorite_games.iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                        recents_valid = config.recent_games.iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                        if added {
                                            overlay_message = Some("ADDED TO FAVORITES".to_string());
                                            play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                        } else {
                                            overlay_message = Some("REMOVED FROM FAVORITES".to_string());
                                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                                        }
                                        overlay_timer = 90;
                                    }
                                }
                                8 => { // Return to menu
                                    netplay.disconnect();
                                    script_engine = None;
                                    if let Some(ref bus) = game_bus {
                                        auto_save_sram(bus, &config);
                                    }
                                    game_bus = None;
                                    game_cpu = None;
                                    paused = false;
                                    quit_hold_frames = 0;
                                    achievement_engine = AchievementEngine::new();
                                    recorder = InputRecording::new([0u8; 32]);
                                    emulator_state = EmulatorState::Menu(MenuState::new());
                                    window_title = format!("OxideNES v{}", env!("OXIDENES_VERSION"));
                                    window.set_title(&window_title);
                                    play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                                    continue;
                                }
                                9 => { // Achievements
                                    achievement_submenu = !achievement_submenu;
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                                10 => { // Save recording
                                    if recorder.frame_count() > 0 {
                                        if let Some(base) = recordings_dir() {
                                            let _ = std::fs::create_dir_all(&base);
                                            let path = base.join(format!("{}.nrec", current_rom_name));
                                            match recorder.save_to_file(path.to_str().unwrap_or("recording.nrec")) {
                                                Ok(()) => {
                                                    overlay_message = Some("RECORDING SAVED".to_string());
                                                    overlay_timer = 90;
                                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                                }
                                                Err(e) => {
                                                    eprintln!("[recording] Save error: {}", e);
                                                    overlay_message = Some("SAVE FAILED".to_string());
                                                    overlay_timer = 90;
                                                    play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                                }
                                            }
                                        }
                                    } else {
                                        overlay_message = Some("NO RECORDING DATA".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    paused = false;
                                }
                                11 => { // Load recording
                                    if let Some(base) = recordings_dir() {
                                        let path = base.join(format!("{}.nrec", current_rom_name));
                                        match InputRecording::load_from_file(path.to_str().unwrap_or("")) {
                                            Ok(loaded) => {
                                                let count = loaded.frame_count();
                                                recorder = loaded;
                                                overlay_message = Some(format!("LOADED {} FRAMES", count));
                                                overlay_timer = 90;
                                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                            }
                                            Err(e) => {
                                                eprintln!("[recording] Load error: {}", e);
                                                overlay_message = Some("LOAD FAILED".to_string());
                                                overlay_timer = 90;
                                                play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                            }
                                        }
                                    }
                                    paused = false;
                                }
                                12 => { // Export FM2
                                    if recorder.frame_count() > 0 {
                                        if let Some(base) = recordings_dir() {
                                            let _ = std::fs::create_dir_all(&base);
                                            let path = base.join(format!("{}.fm2", current_rom_name));
                                            match recorder.export_fm2(path.to_str().unwrap_or("recording.fm2"), &current_rom_name) {
                                                Ok(()) => {
                                                    overlay_message = Some("FM2 EXPORTED".to_string());
                                                    overlay_timer = 90;
                                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                                }
                                                Err(e) => {
                                                    eprintln!("[recording] FM2 export error: {}", e);
                                                    overlay_message = Some("EXPORT FAILED".to_string());
                                                    overlay_timer = 90;
                                                    play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                                }
                                            }
                                        }
                                    } else {
                                        overlay_message = Some("NO RECORDING DATA".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    paused = false;
                                }
                                13 => { // Controls reference page
                                    controls_submenu = true;
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                                _ => {}
                            }
                        }
                        if input.back {
                            paused = false; // ESC again resumes
                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                        }
                    } else {
                        // Normal game emulation when not paused
                        if !quick_overlay {
                        let rewinding = window.is_key_down(Key::Backspace);
                        is_rewinding = rewinding;
                        
                        if rewinding {
                            // Rewind: pop snapshots and render them
                            // Pop 2 frames for visible speed (since we only save every 2nd frame)
                            let _rewound = rewind_buffer.pop_frame(bus, cpu);
                        } else {
                            let fast_forward = window.is_key_down(Key::Tab);
                            let frame_count = if fast_forward { 4 } else { 1 };

                            for ff in 0..frame_count {
                                // Run one frame of emulation
                                loop {
                                    cpu.clock(bus);
                                    bus.tick(1);
                                    bus.tick_apu();
                                    bus.service_dmc_dma();

                                    if bus.ppu.frame_complete() {
                                        break;
                                    }
                                }

                                // End APU frame
                                bus.apu.end_frame();

                                if ff == frame_count - 1 {
                                    std::mem::swap(&mut audio_swap_buf, &mut bus.apu.sample_buffer);
                                    let vol = audio_volume as f32 / 100.0;
                                    for &sample in &audio_swap_buf {
                                        let _ = producer.try_push(sample * vol);
                                    }
                                    audio_swap_buf.clear();
                                } else {
                                    bus.apu.sample_buffer.clear();
                                }
                            }

                            // Save snapshot for rewind (only during normal play, not fast-forward)
                            if !fast_forward {
                                rewind_buffer.push_frame(bus, cpu);
                            }

                            // Fast forward overlay
                            if fast_forward {
                                if overlay_message.is_none() {
                                    overlay_message = Some(">> FAST FORWARD".to_string());
                                }
                                overlay_timer = 2;
                            }

                            // Lua scripting: run per-frame callback
                            if let Some(ref mut script) = script_engine {
                                let ram_snapshot = bus.ram_snapshot();
                                if let Err(e) = script.on_frame(&ram_snapshot, frame_counter as u64) {
                                    eprintln!("[scripting] {}", e);
                                }
                                // Apply overlay pixels onto PPU frame
                                for (x, y, color) in script.overlay_pixels.drain(..) {
                                    if x < 256 && y < 240 {
                                        bus.ppu.frame_data[y * 256 + x] = color;
                                    }
                                }
                                // Show script messages as overlay
                                for (msg, _frames) in script.messages.drain(..) {
                                    overlay_message = Some(msg);
                                    overlay_timer = 2;
                                }
                            }

                            // Achievement system: check RAM conditions each frame
                            {
                                let ram_snapshot = bus.ram_snapshot();
                                achievement_engine.check_frame(&ram_snapshot);
                                achievement_engine.tick_notifications();
                            }
                        }
                        } // end if !quick_overlay
                        
                        // Handle input when not paused
                        frame_counter = frame_counter.wrapping_add(1);
                        let (start_held, select_held, l_held, r_held) = if quick_overlay {
                            // When overlay is open, don't consume gilrs events — let poll_menu_input handle them.
                            // But we still need L+R state for dismissal. Read from raw gamepad state (no event drain).
                            let mut l = false;
                            let mut r = false;
                            if let Some(ref mut g) = gilrs {
                                if let Some((_, gamepad)) = g.gamepads().find(|(_, gp)| gp.is_connected()) {
                                    l = gamepad.is_pressed(Button::LeftTrigger) || gamepad.is_pressed(Button::LeftTrigger2);
                                    r = gamepad.is_pressed(Button::RightTrigger) || gamepad.is_pressed(Button::RightTrigger2);
                                }
                            }
                            (false, false, l, r)
                        } else {
                            handle_input(&window, bus, &mut gilrs, frame_counter, &config.input_bindings, &mut stick_state_p1, &mut stick_state_p2)
                        };

                        // Recording: capture current joypad state after input handling
                        if !quick_overlay {
                            if recorder.is_recording() {
                                let p1 = joypad_to_byte(bus, 1);
                                let p2 = joypad_to_byte(bus, 2);
                                recorder.record_frame(p1, p2);
                            }
                        }

                        // Playback: override joypad input from recording
                        if recorder.is_playing() {
                            if let Some((p1, p2)) = recorder.next_frame() {
                                byte_to_joypad(bus, 1, p1);
                                byte_to_joypad(bus, 2, p2);
                            } else {
                                overlay_message = Some("PLAYBACK FINISHED".to_string());
                                overlay_timer = 90;
                            }
                        }

                        // Netplay: exchange inputs with remote peer
                        if netplay.is_connected() {
                            netplay.frame_num = netplay.frame_num.wrapping_add(1);

                            // Encode local input (from whichever player we are)
                            let local_bits = if netplay.local_player == 0 {
                                // We're P1 - encode our P1 joypad state
                                NetplaySession::encode_input(
                                    bus.joypad1.get_button(JoypadButton::A),
                                    bus.joypad1.get_button(JoypadButton::B),
                                    bus.joypad1.get_button(JoypadButton::Select),
                                    bus.joypad1.get_button(JoypadButton::Start),
                                    bus.joypad1.get_button(JoypadButton::Up),
                                    bus.joypad1.get_button(JoypadButton::Down),
                                    bus.joypad1.get_button(JoypadButton::Left),
                                    bus.joypad1.get_button(JoypadButton::Right),
                                )
                            } else {
                                // We're P2 - encode our local input (read from P1 keys, applied to P2 remotely)
                                NetplaySession::encode_input(
                                    bus.joypad1.get_button(JoypadButton::A),
                                    bus.joypad1.get_button(JoypadButton::B),
                                    bus.joypad1.get_button(JoypadButton::Select),
                                    bus.joypad1.get_button(JoypadButton::Start),
                                    bus.joypad1.get_button(JoypadButton::Up),
                                    bus.joypad1.get_button(JoypadButton::Down),
                                    bus.joypad1.get_button(JoypadButton::Left),
                                    bus.joypad1.get_button(JoypadButton::Right),
                                )
                            };

                            // Simple checksum of frame state
                            let checksum = netplay.frame_num as u32 ^ (local_bits as u32);
                            netplay.send_input(netplay.frame_num, local_bits, checksum);

                            // Receive remote input
                            let remote_bits = netplay.receive_input().unwrap_or(netplay.last_remote_input());
                            let (ra, rb, rsel, rst, rup, rdn, rlt, rrt) = NetplaySession::decode_input(remote_bits);

                            // Apply remote input to the other player's joypad
                            if netplay.local_player == 0 {
                                // We're P1, remote controls P2
                                bus.joypad2.set_button_pressed(JoypadButton::A, ra);
                                bus.joypad2.set_button_pressed(JoypadButton::B, rb);
                                bus.joypad2.set_button_pressed(JoypadButton::Select, rsel);
                                bus.joypad2.set_button_pressed(JoypadButton::Start, rst);
                                bus.joypad2.set_button_pressed(JoypadButton::Up, rup);
                                bus.joypad2.set_button_pressed(JoypadButton::Down, rdn);
                                bus.joypad2.set_button_pressed(JoypadButton::Left, rlt);
                                bus.joypad2.set_button_pressed(JoypadButton::Right, rrt);
                            } else {
                                // We're P2: our local input goes to P2 joypad, remote goes to P1
                                // First, move our local input from joypad1 to joypad2
                                let (la, lb, lsel, lst, lup, ldn, llt, lrt) = NetplaySession::decode_input(local_bits);
                                bus.joypad2.set_button_pressed(JoypadButton::A, la);
                                bus.joypad2.set_button_pressed(JoypadButton::B, lb);
                                bus.joypad2.set_button_pressed(JoypadButton::Select, lsel);
                                bus.joypad2.set_button_pressed(JoypadButton::Start, lst);
                                bus.joypad2.set_button_pressed(JoypadButton::Up, lup);
                                bus.joypad2.set_button_pressed(JoypadButton::Down, ldn);
                                bus.joypad2.set_button_pressed(JoypadButton::Left, llt);
                                bus.joypad2.set_button_pressed(JoypadButton::Right, lrt);
                                // Remote (host) controls P1
                                bus.joypad1.set_button_pressed(JoypadButton::A, ra);
                                bus.joypad1.set_button_pressed(JoypadButton::B, rb);
                                bus.joypad1.set_button_pressed(JoypadButton::Select, rsel);
                                bus.joypad1.set_button_pressed(JoypadButton::Start, rst);
                                bus.joypad1.set_button_pressed(JoypadButton::Up, rup);
                                bus.joypad1.set_button_pressed(JoypadButton::Down, rdn);
                                bus.joypad1.set_button_pressed(JoypadButton::Left, rlt);
                                bus.joypad1.set_button_pressed(JoypadButton::Right, rrt);
                            }
                        } else if netplay.state != NetplayState::Disconnected {
                            // Still hosting/connecting - poll for handshake packets
                            let _ = netplay.receive_input();
                            if netplay.is_connected() {
                                overlay_message = Some(format!("NETPLAY CONNECTED (P{})", netplay.local_player + 1));
                                overlay_timer = 120;
                            }
                        }

                        // Gamepad quit combo: hold Start+Select for ~1 second (60 frames)
                        if start_held && select_held {
                            quit_hold_frames += 1;
                            if quit_hold_frames >= 60 {
                                if let Some(ref bus) = game_bus {
                                    auto_save_sram(bus, &config);
                                }
                                game_bus = None;
                                game_cpu = None;
                                quit_hold_frames = 0;
                                repeat_tracker = RepeatTracker::new();
                                emulator_state = EmulatorState::Menu(MenuState::new());
                                window_title = format!("OxideNES v{}", env!("OXIDENES_VERSION"));
                                window.set_title(&window_title);
                                continue;
                            }
                        } else {
                            quit_hold_frames = 0;
                        }

                        // Slot selection: F2=slot 1, F3=slot 2, F4=slot 3, F6=slot 4
                        if window.is_key_pressed(Key::F2, KeyRepeat::No) {
                            current_save_slot = 1;
                            pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                            pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                            overlay_message = Some("SLOT 1 SELECTED".to_string());
                            overlay_timer = 60;
                        }
                        if window.is_key_pressed(Key::F3, KeyRepeat::No) {
                            current_save_slot = 2;
                            pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                            pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                            overlay_message = Some("SLOT 2 SELECTED".to_string());
                            overlay_timer = 60;
                        }
                        if window.is_key_pressed(Key::F4, KeyRepeat::No) {
                            current_save_slot = 3;
                            pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                            pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                            overlay_message = Some("SLOT 3 SELECTED".to_string());
                            overlay_timer = 60;
                        }
                        if window.is_key_pressed(Key::F6, KeyRepeat::No) {
                            current_save_slot = 4;
                            pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                            pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                            overlay_message = Some("SLOT 4 SELECTED".to_string());
                            overlay_timer = 60;
                        }

                        // F5 quick save, F9 quick load (using current slot)
                        if window.is_key_pressed(Key::F5, KeyRepeat::No) {
                            if save_state(bus, cpu, &config, current_save_slot) {
                                overlay_message = Some(format!("STATE {} SAVED", current_save_slot));
                                overlay_timer = 90;
                            } else {
                                overlay_message = Some("SAVE FAILED".to_string());
                                overlay_timer = 90;
                            }
                        }
                        if window.is_key_pressed(Key::F9, KeyRepeat::No) {
                            if load_state(bus, cpu, &config, current_save_slot) {
                                overlay_message = Some(format!("STATE {} LOADED", current_save_slot));
                                overlay_timer = 90;
                            } else {
                                overlay_message = Some("NO SAVE FOUND".to_string());
                                overlay_timer = 90;
                            }
                        }

                        // F8 screenshot
                        if window.is_key_pressed(Key::F8, KeyRepeat::No) {
                            if let Some(_path) = save_screenshot(&bus.ppu.frame_data) {
                                overlay_message = Some("SCREENSHOT SAVED".to_string());
                                overlay_timer = 90;
                            } else {
                                overlay_message = Some("SCREENSHOT FAILED".to_string());
                                overlay_timer = 90;
                            }
                        }

                        // F10 FPS counter toggle
                        if window.is_key_pressed(Key::F10, KeyRepeat::No) {
                            show_fps = !show_fps;
                            if !show_fps {
                                fps_display.clear();
                            }
                        }

                        // F11 fullscreen toggle
                        if window.is_key_pressed(Key::F11, KeyRepeat::No) {
                            is_fullscreen = !is_fullscreen;
                            if is_fullscreen {
                                let (sw, sh) = get_screen_resolution();
                                window = Window::new(
                                    &window_title,
                                    sw,
                                    sh,
                                    WindowOptions {
                                        borderless: true,
                                        scale: Scale::X1,
                                        scale_mode: ScaleMode::AspectRatioStretch,
                                        ..WindowOptions::default()
                                    },
                                ).expect("Failed to create fullscreen window");
                                window.set_position(0, 0);
                            } else {
                                window = Window::new(
                                    &window_title,
                                    WINDOW_WIDTH,
                                    WINDOW_HEIGHT,
                                    WindowOptions {
                                        scale: Scale::X1,
                                        ..WindowOptions::default()
                                    },
                                ).expect("Failed to create window");
                            }
                            window.set_target_fps(target_fps);
                            overlay_message = Some(if is_fullscreen { "FULLSCREEN".to_string() } else { "WINDOWED".to_string() });
                            overlay_timer = 60;
                        }

                        // F12 help overlay
                        if window.is_key_pressed(Key::F12, KeyRepeat::No) {
                            show_help = !show_help;
                        }

                        // F7 reset game
                        if window.is_key_pressed(Key::F7, KeyRepeat::No) {
                            cpu.reset(bus);
                            overlay_message = Some("GAME RESET".to_string());
                            overlay_timer = 90;
                        }

                        // M mute toggle
                        if window.is_key_pressed(Key::M, KeyRepeat::No) {
                            if audio_volume > 0 {
                                config.audio_volume = audio_volume;
                                save_config(&config);
                                audio_volume = 0;
                                overlay_message = Some("VOLUME: [..........] MUTED".to_string());
                            } else {
                                audio_volume = config.audio_volume;
                                if audio_volume == 0 { audio_volume = 100; }
                                let bars = (audio_volume / 10) as usize;
                                let bar: String = "#".repeat(bars) + &".".repeat(10 - bars);
                                overlay_message = Some(format!("VOLUME: [{}] {}%", bar, audio_volume));
                            }
                            overlay_timer = 60;
                        }

                        if window.is_key_pressed(Key::F1, KeyRepeat::No) {
                            crt_enabled = !crt_enabled;
                            config.crt_enabled = crt_enabled;
                            save_config(&config);
                            overlay_message = Some(if crt_enabled { "CRT FILTER: ON".to_string() } else { "CRT FILTER: OFF".to_string() });
                            overlay_timer = 90; // 1.5 seconds
                        }

                        // Shift+R toggle recording
                        if window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift) {
                            if window.is_key_pressed(Key::R, KeyRepeat::No) {
                                if recorder.is_recording() {
                                    recorder.stop_recording();
                                    overlay_message = Some(format!("REC STOPPED ({} FRAMES)", recorder.frame_count()));
                                    overlay_timer = 90;
                                } else {
                                    recorder.start_recording();
                                    overlay_message = Some("REC STARTED".to_string());
                                    overlay_timer = 90;
                                }
                            }
                            // Shift+P toggle playback
                            if window.is_key_pressed(Key::P, KeyRepeat::No) {
                                if recorder.is_playing() {
                                    recorder.stop_recording(); // stops playback (sets Idle)
                                    overlay_message = Some("PLAYBACK STOPPED".to_string());
                                    overlay_timer = 90;
                                } else if recorder.frame_count() > 0 {
                                    recorder.start_playback();
                                    overlay_message = Some(format!("PLAYING {} FRAMES", recorder.frame_count()));
                                    overlay_timer = 90;
                                } else {
                                    overlay_message = Some("NO RECORDING".to_string());
                                    overlay_timer = 90;
                                }
                            }
                        }

                        // L+R Quick Overlay toggle
                        if l_held && r_held {
                            quick_overlay_lr_frames += 1;
                            if quick_overlay_lr_frames == 20 && !quick_overlay {
                                // Open overlay after 20 frames debounce (~333ms)
                                quick_overlay = true;
                                quick_overlay_selected = 0;
                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                            }
                        } else {
                            if quick_overlay_lr_frames > 0 && quick_overlay_lr_frames < 20 {
                                // Short tap — ignore
                            }
                            quick_overlay_lr_frames = 0;
                        }

                        // Quick overlay input handling
                        if quick_overlay {
                            let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker, config.input_bindings.controller_p1.deadzone, &mut stick_state_menu);
                            let overlay_item_count: usize = 6;

                            if input.up && quick_overlay_selected > 0 {
                                quick_overlay_selected -= 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            if input.down && quick_overlay_selected < overlay_item_count - 1 {
                                quick_overlay_selected += 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                    sound_cooldown = 3;
                                }
                            }
                            // L/R cycles save slot on save/load items
                            if quick_overlay_selected == 1 || quick_overlay_selected == 2 {
                                if input.left {
                                    current_save_slot = if current_save_slot == 1 { 5 } else { current_save_slot - 1 };
                                    pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                                    pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                                }
                                if input.right {
                                    current_save_slot = if current_save_slot == 5 { 1 } else { current_save_slot + 1 };
                                    pause_save_label = format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                                    pause_load_label = format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                                }
                            }
                            if input.confirm {
                                match quick_overlay_selected {
                                    0 => { // Resume
                                        quick_overlay = false;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    1 => { // Save State
                                        if save_state(bus, cpu, &config, current_save_slot) {
                                            thumbnail_cache[(current_save_slot as usize).saturating_sub(1).min(3)] = load_thumbnail(&config, current_save_slot);
                                            overlay_message = Some(format!("STATE {} SAVED", current_save_slot));
                                            overlay_timer = 90;
                                        } else {
                                            overlay_message = Some("SAVE FAILED".to_string());
                                            overlay_timer = 90;
                                        }
                                        quick_overlay = false;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    2 => { // Load State
                                        if load_state(bus, cpu, &config, current_save_slot) {
                                            overlay_message = Some(format!("STATE {} LOADED", current_save_slot));
                                            overlay_timer = 90;
                                        } else {
                                            overlay_message = Some("NO SAVE FOUND".to_string());
                                            overlay_timer = 90;
                                        }
                                        quick_overlay = false;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    3 => { // Toggle Favorite
                                        let added = toggle_favorite(&mut config, &current_rom_path);
                                        save_config(&config);
                                        favorites_valid = config.favorite_games.iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists()).collect();
                                        if added {
                                            overlay_message = Some("ADDED TO FAVORITES".to_string());
                                        } else {
                                            overlay_message = Some("REMOVED FROM FAVORITES".to_string());
                                        }
                                        overlay_timer = 90;
                                        quick_overlay = false;
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    4 => { // Full Pause Menu
                                        quick_overlay = false;
                                        paused = true;
                                        pause_selected = 0;
                                        pause_cursor_timer = 0;
                                        pause_cursor_visible = true;
                                        for slot in 0..4u8 {
                                            thumbnail_cache[slot as usize] = load_thumbnail(&config, slot + 1);
                                        }
                                        play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    }
                                    5 => { // Return to Menu
                                        if let Some(ref bus) = game_bus {
                                            auto_save_sram(bus, &config);
                                        }
                                        game_bus = None;
                                        game_cpu = None;
                                        quick_overlay = false;
                                        repeat_tracker = RepeatTracker::new();
                                        emulator_state = EmulatorState::Menu(MenuState::new());
                                        window_title = format!("OxideNES v{}", env!("OXIDENES_VERSION"));
                                        window.set_title(&window_title);
                                        play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                                        continue;
                                    }
                                    _ => {}
                                }
                            }
                            if input.back {
                                quick_overlay = false;
                                play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            }
                        }

                        // Escape toggles pause menu
                        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                            paused = true;
                            pause_selected = 0;
                            pause_cursor_timer = 0;
                            pause_cursor_visible = true;
                            // Load thumbnails for save slot display
                            for slot in 0..4u8 {
                                thumbnail_cache[slot as usize] = load_thumbnail(&config, slot + 1);
                            }
                        }
                    }

                    // ALWAYS render (even when paused - shows frozen frame)
                    let dt = if barrel_distortion { &distortion_table } else { &flat_distortion_table };
                    if crt_enabled {
                        crt_filter(&bus.ppu.frame_data, &mut crt_buffer, &vignette_table, dt, &config.crt_config, &mask_table);
                        // Phosphor bloom — bright pixels glow into neighbors
                        apply_phosphor_bloom(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                        apply_scanline_glow(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                        // Apply chromatic aberration to crt_buffer (screen area only)
                        if glass_intensity > 30 {
                            ca_temp.copy_from_slice(&crt_buffer[..SCREEN_W * SCREEN_H]);
                            apply_chromatic_aberration(&mut crt_buffer, &ca_temp, &ca_table, SCREEN_W, SCREEN_H);
                        }
                    } else {
                        scale_simple(&bus.ppu.frame_data, &mut crt_buffer);
                    }

                    // Composite game output into TV frame
                    composite_screen_fast(&mut composite_buffer, &crt_buffer, WINDOW_WIDTH);

                    if crt_enabled {
                        apply_screen_glare(&mut composite_buffer, &glare_table, &glass_thickness_table, WINDOW_WIDTH, glass_intensity);
                        // Internal ghost reflection from thick CRT glass
                        if glass_intensity > 20 {
                            for y in 0..SCREEN_H {
                                let row_start = (y + SCREEN_Y) * WINDOW_WIDTH + SCREEN_X;
                                ghost_buffer[row_start..row_start + SCREEN_W]
                                    .copy_from_slice(&composite_buffer[row_start..row_start + SCREEN_W]);
                            }
                            apply_internal_ghost(&mut composite_buffer, &ghost_buffer, &ghost_alpha_table, WINDOW_WIDTH);
                        }
                    }
                    if is_rewinding {
                        let sx = SCREEN_X;
                        let sy = SCREEN_Y;
                        let sw = SCREEN_W;
                        let sh = SCREEN_H;
                        let fc = frame_counter as u64;

                        // Deterministic pseudo-random hash (no rand crate)
                        #[inline(always)]
                        fn vhs_hash(x: usize, y: usize, f: u64) -> u32 {
                            (x as u32).wrapping_mul(2654435761)
                                ^ (y as u32).wrapping_mul(340573321)
                                ^ (f as u32).wrapping_mul(1013904223)
                        }

                        // ── 5. Brightness pumping: triangle wave 0.70–0.94 ──
                        let pump = (fc % 120) as u32;
                        let pump = if pump < 60 { pump } else { 120 - pump };
                        let bright = 180 + pump; // numerator out of 256

                        // ── Scanline loop (tearing → color bleed → per-pixel) ──
                        for row in 0..sh {
                            let buf_y = sy + row;

                            // ── 2. Horizontal tearing: groups of ~12 lines shift ──
                            let tear_seed = vhs_hash(0, row / 12, fc.wrapping_mul(3));
                            let tear = (tear_seed % 47) as i32 - 23; // −23..+23 px
                            if tear > 0 {
                                let t = tear as usize;
                                for x in (t..sw).rev() {
                                    let d = buf_y * WINDOW_WIDTH + sx + x;
                                    let s = d - t;
                                    if d < composite_buffer.len() && s < composite_buffer.len() {
                                        composite_buffer[d] = composite_buffer[s];
                                    }
                                }
                                for x in 0..t.min(sw) {
                                    let i = buf_y * WINDOW_WIDTH + sx + x;
                                    if i < composite_buffer.len() {
                                        let n = vhs_hash(x, row, fc) & 0x3F;
                                        composite_buffer[i] = (n << 16) | (n << 8) | n;
                                    }
                                }
                            } else if tear < 0 {
                                let t = (-tear) as usize;
                                for x in 0..sw.saturating_sub(t) {
                                    let d = buf_y * WINDOW_WIDTH + sx + x;
                                    let s = d + t;
                                    if d < composite_buffer.len() && s < composite_buffer.len() {
                                        composite_buffer[d] = composite_buffer[s];
                                    }
                                }
                                for x in sw.saturating_sub(t)..sw {
                                    let i = buf_y * WINDOW_WIDTH + sx + x;
                                    if i < composite_buffer.len() {
                                        let n = vhs_hash(x, row, fc) & 0x3F;
                                        composite_buffer[i] = (n << 16) | (n << 8) | n;
                                    }
                                }
                            }

                            // ── 3. Color bleeding on ~30 % of scanlines ──
                            let bleed_h = vhs_hash(0, row, fc / 2);
                            if bleed_h % 10 < 3 {
                                let sp = 1 + (bleed_h as usize % 2); // 1–2 px
                                // Shift red channel left (source = right neighbour)
                                for x in 0..sw.saturating_sub(sp) {
                                    let di = buf_y * WINDOW_WIDTH + sx + x;
                                    let si = di + sp;
                                    if di < composite_buffer.len() && si < composite_buffer.len() {
                                        let src_r = (composite_buffer[si] >> 16) & 0xFF;
                                        composite_buffer[di] = (src_r << 16) | (composite_buffer[di] & 0x00FFFF);
                                    }
                                }
                                // Shift blue channel right (source = left neighbour)
                                for x in (sp..sw).rev() {
                                    let di = buf_y * WINDOW_WIDTH + sx + x;
                                    let si = di - sp;
                                    if di < composite_buffer.len() && si < composite_buffer.len() {
                                        let src_b = composite_buffer[si] & 0xFF;
                                        composite_buffer[di] = (composite_buffer[di] & 0xFFFF00) | src_b;
                                    }
                                }
                            }

                            // ── Per-pixel: desaturate, pump, snow, roll bar, speed lines ──
                            for x in 0..sw {
                                let idx = buf_y * WINDOW_WIDTH + sx + x;
                                if idx >= composite_buffer.len() { break; }
                                let p = composite_buffer[idx];
                                let mut r = (p >> 16) & 0xFF;
                                let mut g = (p >> 8) & 0xFF;
                                let mut b = p & 0xFF;

                                // Desaturate + blue tint
                                let gray = (r * 77 + g * 150 + b * 29) >> 8;
                                r = (r * 55 + gray * 45) / 100;
                                g = (g * 55 + gray * 45) / 100;
                                b = ((b * 55 + gray * 45) / 100) * 135 / 100;

                                // Brightness pumping
                                r = (r * bright) >> 8;
                                g = (g * bright) >> 8;
                                b = (b * bright) >> 8;

                                // ── 4. Static / snow (heavier at top & bottom edges) ──
                                let noise = vhs_hash(x, row, fc);
                                let edge = if row < 40 || row + 40 > sh { 3u32 } else { 0 };
                                if (noise & 0x07) < 1 + edge {
                                    let snow = (noise >> 3) & 0xFF;
                                    r = (r * 60 + snow * 40) / 100;
                                    g = (g * 60 + snow * 40) / 100;
                                    b = (b * 60 + snow * 40) / 100;
                                }

                                // ── 6. Rolling dark bar (~40 px, scrolls upward) ──
                                let roll_y = ((fc.wrapping_mul(3)) % sh as u64) as usize;
                                let dist = if row >= roll_y { row - roll_y } else { row + sh - roll_y };
                                if dist < 40 {
                                    let fade = if dist < 10 { 128u32 } else if dist < 30 { 160 } else { 192 };
                                    r = (r * fade) >> 8;
                                    g = (g * fade) >> 8;
                                    b = (b * fade) >> 8;
                                }

                                // ── 8. Diagonal speed lines (3 faint, ~20 % white) ──
                                for ln in 0..3u64 {
                                    let diag = (x as u64 + row as u64 + fc * 11 + ln * 257) % 200;
                                    if diag < 2 {
                                        r = r + (255 - r) / 5;
                                        g = g + (255 - g) / 5;
                                        b = b + (255 - b) / 5;
                                    }
                                }

                                composite_buffer[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
                            }
                        }

                        // ── 1. Tracking lines: 4 bright noise bands scrolling down ──
                        for band in 0..4u64 {
                            let by = ((fc.wrapping_mul(7).wrapping_add(band * 193)) % sh as u64) as usize;
                            for dy in 0..3usize {
                                let row = (by + dy) % sh;
                                let buf_y = sy + row;
                                for x in 0..sw {
                                    let idx = buf_y * WINDOW_WIDTH + sx + x;
                                    if idx < composite_buffer.len() {
                                        let n = vhs_hash(x, row, fc) & 0xFF;
                                        let v = 128 + n / 2; // bright static 128..255
                                        composite_buffer[idx] = (v << 16) | (v << 8) | v;
                                    }
                                }
                            }
                        }
                    }

                    // Rewind buffer HUD bar (VCR style, top-right)
                    if !paused && is_rewinding && !rewind_buffer.snapshots.is_empty() {
                        let rewind_pct = (rewind_buffer.snapshots.len() * 100) / rewind_buffer.max_snapshots;
                        let bar_w: usize = 60;
                        let bar_h: usize = 4;
                        let bar_x = SCREEN_X + SCREEN_W - 70;
                        let bar_y = SCREEN_Y + 8;
                        let filled = (bar_w * rewind_pct) / 100;
                        for dy in 0..bar_h {
                            for dx in 0..bar_w {
                                let px = bar_x + dx;
                                let py = bar_y + dy;
                                let idx = py * WINDOW_WIDTH + px;
                                if idx < composite_buffer.len() {
                                    composite_buffer[idx] = if dx < filled { 0x00DDDD } else { 0x102030 };
                                }
                            }
                        }
                    }

                    // ── 7. VCR on-screen display: "<< REW" + backward timecode ──
                    if is_rewinding {
                        let fc = frame_counter as u64;
                        let osd_x = SCREEN_X + SCREEN_W - 100;
                        let osd_y = SCREEN_Y + SCREEN_H - 30;
                        // Semi-transparent dark background box
                        for y in osd_y.saturating_sub(3)..=(osd_y + 18) {
                            for x in osd_x.saturating_sub(4)..=(osd_x + 96) {
                                if y < WINDOW_HEIGHT && x < WINDOW_WIDTH {
                                    let idx = y * WINDOW_WIDTH + x;
                                    if idx < composite_buffer.len() {
                                        let p = composite_buffer[idx];
                                        composite_buffer[idx] = ((((p >> 16) & 0xFF) / 4) << 16)
                                            | ((((p >> 8) & 0xFF) / 4) << 8)
                                            | ((p & 0xFF) / 4);
                                    }
                                }
                            }
                        }
                        draw_text(&mut composite_buffer, "<< REW", osd_x, osd_y, 0x00FFFF, WINDOW_WIDTH);
                        // Fake backward-counting timecode  MM:SS:FF
                        let tc_total = 36000u64.saturating_sub(fc % 36000);
                        let display_s = tc_total / 60;
                        let mm = (display_s / 60) % 100;
                        let ss = display_s % 60;
                        let ff = tc_total % 60;
                        let tc = format!("{:02}:{:02}:{:02}", mm, ss, ff);
                        draw_text(&mut composite_buffer, &tc, osd_x + 8, osd_y + 10, 0x00DDDD, WINDOW_WIDTH);
                    }

                    // Overlay message display
                    if overlay_timer > 0 {
                        overlay_timer -= 1;
                        if let Some(ref msg) = overlay_message {
                            let text_w = msg.len() * 4; // approximate pixel width at small scale
                            let ox = SCREEN_X + SCREEN_W / 2 - text_w;
                            let oy = SCREEN_Y + 20;
                            // Dark background bar
                            for y in oy.saturating_sub(2)..=(oy + 8) {
                                for x in (ox.saturating_sub(4))..=(ox + text_w * 2 + 4) {
                                    if y < WINDOW_HEIGHT && x < WINDOW_WIDTH {
                                        let idx = y * WINDOW_WIDTH + x;
                                        if idx < composite_buffer.len() {
                                            composite_buffer[idx] = 0x222222;
                                        }
                                    }
                                }
                            }
                            draw_text(&mut composite_buffer, msg, ox, oy, 0x44FF44, WINDOW_WIDTH);
                        }
                        if overlay_timer == 0 {
                            overlay_message = None;
                        }
                    }

                    // Achievement notification toasts (top-right, gold background)
                    {
                        let mut notify_y = SCREEN_Y + 40;
                        for notif in achievement_engine.notifications.iter() {
                            if notif.frames_remaining == 0 { continue; }
                            let text = &notif.cached_text;
                            let text_w = text.len() * 4;
                            let nx = SCREEN_X + SCREEN_W - text_w - 16;
                            // Gold background bar
                            for y in notify_y.saturating_sub(2)..=(notify_y + 8) {
                                for x in (nx.saturating_sub(4))..=(nx + text_w + 4) {
                                    if y < WINDOW_HEIGHT && x < WINDOW_WIDTH {
                                        let idx = y * WINDOW_WIDTH + x;
                                        if idx < composite_buffer.len() {
                                            composite_buffer[idx] = 0x886820;
                                        }
                                    }
                                }
                            }
                            draw_text(&mut composite_buffer, &text, nx, notify_y, 0xF8D878, WINDOW_WIDTH);
                            notify_y += 14;
                        }
                    }

                    // Recording/playback HUD indicators (top-left)
                    if !paused {
                        if recorder.is_recording() {
                            let rec_text = "* REC";
                            let rx = SCREEN_X + 8;
                            let ry = SCREEN_Y + 20;
                            // Dark background
                            let tw = rec_text.len() * 4 + 4;
                            for y in ry.saturating_sub(1)..=(ry + 6) {
                                for x in rx.saturating_sub(2)..=(rx + tw) {
                                    if y < WINDOW_HEIGHT && x < WINDOW_WIDTH {
                                        let idx = y * WINDOW_WIDTH + x;
                                        if idx < composite_buffer.len() {
                                            let p = composite_buffer[idx];
                                            let r = ((p >> 16) & 0xFF) / 3;
                                            let g = ((p >> 8) & 0xFF) / 3;
                                            let b = (p & 0xFF) / 3;
                                            composite_buffer[idx] = (r << 16) | (g << 8) | b;
                                        }
                                    }
                                }
                            }
                            draw_text(&mut composite_buffer, rec_text, rx, ry, 0xFF4444, WINDOW_WIDTH);
                        }
                        if recorder.is_playing() {
                            let play_text = "> PLAY";
                            let px = SCREEN_X + 8;
                            let py = SCREEN_Y + 20;
                            let tw = play_text.len() * 4 + 4;
                            for y in py.saturating_sub(1)..=(py + 6) {
                                for x in px.saturating_sub(2)..=(px + tw) {
                                    if y < WINDOW_HEIGHT && x < WINDOW_WIDTH {
                                        let idx = y * WINDOW_WIDTH + x;
                                        if idx < composite_buffer.len() {
                                            let p = composite_buffer[idx];
                                            let r = ((p >> 16) & 0xFF) / 3;
                                            let g = ((p >> 8) & 0xFF) / 3;
                                            let b = (p & 0xFF) / 3;
                                            composite_buffer[idx] = (r << 16) | (g << 8) | b;
                                        }
                                    }
                                }
                            }
                            draw_text(&mut composite_buffer, play_text, px, py, 0x44FF44, WINDOW_WIDTH);
                        }
                    }

                    // FPS counter
                    if show_fps {
                        fps_frames += 1;
                        let elapsed = fps_timer.elapsed().as_secs_f64();
                        if elapsed >= 1.0 {
                            fps_display = format!("FPS: {}", fps_frames);
                            fps_frames = 0;
                            fps_timer = std::time::Instant::now();
                        }
                        if !fps_display.is_empty() {
                            let fx = SCREEN_X + SCREEN_W - fps_display.len() * 8 - 8;
                            let fy = SCREEN_Y + 8;
                            draw_text(&mut composite_buffer, &fps_display, fx, fy, 0x00FF00, WINDOW_WIDTH);
                        }
                    }

                    // Netplay indicator (top-left during gameplay)
                    if netplay.is_connected() && !paused {
                        if netplay.ping_ms != cached_net_ping {
                            cached_net_ping = netplay.ping_ms;
                            cached_net_text = format!("NET {}MS", netplay.ping_ms);
                        }
                        let nx = SCREEN_X + 8;
                        let ny = SCREEN_Y + 8;
                        // Dark background
                        let tw = cached_net_text.len() * 4 + 4;
                        for y in ny.saturating_sub(1)..=(ny + 6) {
                            for x in nx.saturating_sub(2)..=(nx + tw) {
                                if y < WINDOW_HEIGHT && x < WINDOW_WIDTH {
                                    let idx = y * WINDOW_WIDTH + x;
                                    if idx < composite_buffer.len() {
                                        let p = composite_buffer[idx];
                                        let r = ((p >> 16) & 0xFF) / 3;
                                        let g = ((p >> 8) & 0xFF) / 3;
                                        let b = (p & 0xFF) / 3;
                                        composite_buffer[idx] = (r << 16) | (g << 8) | b;
                                    }
                                }
                            }
                        }
                        draw_text(&mut composite_buffer, &cached_net_text, nx, ny, 0x44CCFF, WINDOW_WIDTH);
                    }

                    // Help overlay
                    if show_help {
                        // Semi-transparent dark background over the screen area
                        for y in SCREEN_Y..(SCREEN_Y + SCREEN_H) {
                            for x in SCREEN_X..(SCREEN_X + SCREEN_W) {
                                let idx = y * WINDOW_WIDTH + x;
                                if idx < composite_buffer.len() {
                                    let p = composite_buffer[idx];
                                    let r = ((p >> 16) & 0xFF) / 3;
                                    let g = ((p >> 8) & 0xFF) / 3;
                                    let b = (p & 0xFF) / 3;
                                    composite_buffer[idx] = (r << 16) | (g << 8) | b;
                                }
                            }
                        }
                        
                        let help_x = SCREEN_X + 40;
                        let mut help_y = SCREEN_Y + 30;
                        let color = 0x44FF44;
                        let dim = 0x888888;
                        
                        draw_text(&mut composite_buffer, "=== CONTROLS ===", help_x, help_y, 0xFFFF00, WINDOW_WIDTH);
                        help_y += 20;
                        draw_text(&mut composite_buffer, "WASD/ARROWS  D-PAD", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "K  A BUTTON", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "J  B BUTTON", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "ENTER  START", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "RSHIFT  SELECT", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "Z/X  TURBO A/B", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 24;
                        draw_text(&mut composite_buffer, "=== EMULATOR ===", help_x, help_y, 0xFFFF00, WINDOW_WIDTH);
                        help_y += 20;
                        draw_text(&mut composite_buffer, "ESC  PAUSE MENU", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "TAB  FAST FORWARD", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "BACKSPACE  REWIND", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "M  MUTE/UNMUTE", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "F1  CRT FILTER", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "F2-F4,F6  SAVE SLOT", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "F5/F9  SAVE/LOAD", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "F7  RESET GAME", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "F8  SCREENSHOT", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "F10  FPS COUNTER", help_x, help_y, color, WINDOW_WIDTH);
                        help_y += 12;
                        draw_text(&mut composite_buffer, "F12  THIS HELP", help_x, help_y, dim, WINDOW_WIDTH);
                        help_y += 24;
                        draw_text(&mut composite_buffer, "PRESS F12 TO CLOSE", help_x, help_y, dim, WINDOW_WIDTH);
                    }

                    // Draw quit combo progress overlay (uses previous frame's counter)
                    if quit_hold_frames > 0 {
                        let progress = quit_hold_frames as f32 / 60.0;
                        let bar_w: usize = 200;
                        let bar_h: usize = 8;
                        let bar_x = SCREEN_X + (SCREEN_W - bar_w) / 2;
                        let bar_y = SCREEN_Y + SCREEN_H - 40;
                        let text_x = bar_x + (bar_w - 17 * 4) / 2;
                        let text_y = bar_y - 10;
                        draw_text(&mut composite_buffer, "RETURNING TO MENU", text_x, text_y, 0xFFFFFF, WINDOW_WIDTH);
                        for y in bar_y..bar_y + bar_h {
                            for x in bar_x..bar_x + bar_w {
                                if y * WINDOW_WIDTH + x < composite_buffer.len() {
                                    composite_buffer[y * WINDOW_WIDTH + x] = 0x333333;
                                }
                            }
                        }
                        let fill_w = (bar_w as f32 * progress) as usize;
                        for y in bar_y..bar_y + bar_h {
                            for x in bar_x..bar_x + fill_w {
                                if y * WINDOW_WIDTH + x < composite_buffer.len() {
                                    composite_buffer[y * WINDOW_WIDTH + x] = 0x00FF00;
                                }
                            }
                        }
                    }
                    
                    // NES-style pause menu rendering
                    if paused {
                        // Update pause cursor blink (~500ms at 60fps)
                        pause_cursor_timer += 1;
                        if pause_cursor_timer >= 30 {
                            pause_cursor_timer = 0;
                            pause_cursor_visible = !pause_cursor_visible;
                        }

                        // Copy and darken the last game frame into menu_framebuffer
                        for i in 0..menu_framebuffer.len().min(bus.ppu.frame_data.len()) {
                            let p = bus.ppu.frame_data[i];
                            let r = ((p >> 16) & 0xFF) / 3;
                            let g = ((p >> 8) & 0xFF) / 3;
                            let b = (p & 0xFF) / 3;
                            menu_framebuffer[i] = (r << 16) | (g << 8) | b;
                        }
                        
                        // Box background (tile coordinates: 32 cols × 30 rows)
                        // Center a box roughly 20 tiles wide × 22 tiles tall
                        let box_left = 6;
                        let box_right = 26;
                        let box_top = 3;
                        let box_bottom = 28;
                        
                        // Fill box background
                        for ty in box_top..box_bottom {
                            for tx in box_left..box_right {
                                let px = tx * 8;
                                let py = ty * 8;
                                for dy in 0..8 {
                                    for dx in 0..8 {
                                        let x = px + dx;
                                        let y = py + dy;
                                        if y < 240 && x < 256 {
                                            menu_framebuffer[y * 256 + x] = MENU_BG;
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Draw border using MENU_LIGHT_BLUE
                        for tx in box_left..box_right {
                            let px = tx * 8;
                            for dx in 0..8 {
                                let x = px + dx;
                                // Top border
                                let y_top = box_top * 8;
                                if y_top < 240 && x < 256 {
                                    menu_framebuffer[y_top * 256 + x] = MENU_LIGHT_BLUE;
                                    menu_framebuffer[(y_top + 1) * 256 + x] = MENU_LIGHT_BLUE;
                                }
                                // Bottom border
                                let y_bot = box_bottom * 8 - 1;
                                if y_bot < 240 && x < 256 {
                                    menu_framebuffer[y_bot * 256 + x] = MENU_LIGHT_BLUE;
                                    menu_framebuffer[(y_bot - 1) * 256 + x] = MENU_LIGHT_BLUE;
                                }
                            }
                        }
                        for ty in box_top..box_bottom {
                            let py = ty * 8;
                            for dy in 0..8 {
                                let y = py + dy;
                                if y < 240 {
                                    // Left border
                                    let xl = box_left * 8;
                                    menu_framebuffer[y * 256 + xl] = MENU_LIGHT_BLUE;
                                    menu_framebuffer[y * 256 + xl + 1] = MENU_LIGHT_BLUE;
                                    // Right border
                                    let xr = box_right * 8 - 1;
                                    if xr < 256 {
                                        menu_framebuffer[y * 256 + xr] = MENU_LIGHT_BLUE;
                                        menu_framebuffer[y * 256 + xr - 1] = MENU_LIGHT_BLUE;
                                    }
                                }
                            }
                        }
                        
                        // Title: game name or "PAUSED" centered
                        if !current_rom_name.is_empty() {
                            let title: String = current_rom_name.to_uppercase().chars().take(24).collect();
                            draw_text_centered_8x8(&mut menu_framebuffer, &format!("\x11 {} \x11", title), box_top + 1, MENU_GOLD);
                        } else {
                            draw_text_centered_8x8(&mut menu_framebuffer, "\x11 PAUSED \x11", box_top + 1, MENU_GOLD);
                        }
                        
                        // Separator
                        let sep_y = (box_top + 2) * 8 + 4;
                        for x in (box_left * 8 + 8)..(box_right * 8 - 8) {
                            if x % 4 < 2 && sep_y < 240 {
                                menu_framebuffer[sep_y * 256 + x] = MENU_DARK_GRAY;
                            }
                        }
                        
                        // Menu items (use cached slot labels)
                        let net_status = match &netplay.state {
                            NetplayState::Connected => format!("NETPLAY  ({}MS)", netplay.ping_ms),
                            NetplayState::Hosting { .. } => "NETPLAY  (HOSTING)".to_string(),
                            NetplayState::Connecting => "NETPLAY  (...)".to_string(),
                            _ => "NETPLAY".to_string(),
                        };
                        let script_status = if script_engine.as_ref().map_or(false, |s| s.active) {
                            "RELOAD SCRIPT"
                        } else {
                            "LOAD SCRIPT"
                        };
                        let ach_label = if achievement_engine.achievements.is_empty() {
                            "ACHIEVEMENTS".to_string()
                        } else {
                            format!("ACHIEVEMENTS ({}/{})", achievement_engine.unlocked_count, achievement_engine.achievements.len())
                        };
                        let rec_label = if recorder.is_recording() {
                            format!("SAVE REC ({} FR)", recorder.frame_count())
                        } else {
                            "SAVE RECORDING".to_string()
                        };
                        let cheat_label = format!("CHEATS ({})", bus.cheats.len());
                        let fav_label = if is_favorite(&config, &current_rom_path) {
                            "REMOVE FROM FAVORITES".to_string()
                        } else {
                            "ADD TO FAVORITES".to_string()
                        };
                        let items: Vec<&str> = vec!["RESUME GAME", &pause_save_label, &pause_load_label, &cheat_label, &net_status, script_status, "UNLOAD SCRIPT", &fav_label, "RETURN TO MENU", &ach_label, &rec_label, "LOAD RECORDING", "EXPORT FM2", "CONTROLS"];
                        for (i, item) in items.iter().enumerate() {
                            let row = box_top + 3 + i;
                            let is_selected = i == pause_selected;
                            
                            if is_selected {
                                // Highlight bar (always visible)
                                draw_highlight_bar(&mut menu_framebuffer, row * 8, 8, box_left * 8 + 4, box_right * 8 - 4, 0x3C3C8C);
                                // Arrow blinks
                                if pause_cursor_visible {
                                    draw_char_8x8(&mut menu_framebuffer, '\x10', box_left + 1, row, MENU_WHITE);
                                }
                            }
                            
                            let color = if is_selected { MENU_WHITE } else { MENU_GRAY };
                            draw_text_8x8(&mut menu_framebuffer, item, box_left + 2, row, color);
                        }
                        
                        // Hint at bottom of box
                        if pause_selected == 1 || pause_selected == 2 {
                            draw_text_centered_8x8(&mut menu_framebuffer, "A:SELECT  L/R:SLOT", box_bottom - 1, MENU_DARK_GRAY);
                        } else {
                            draw_text_centered_8x8(&mut menu_framebuffer, "ESC:RESUME  A:SELECT", box_bottom - 1, MENU_DARK_GRAY);
                        }
                        
                        if cheats_submenu {
                            // Cheats submenu overlay
                            let cb_left = 4;
                            let cb_right = 28;
                            let cb_top = 6;
                            let cb_bottom = 24;
                            // Background fill
                            for ty in cb_top..cb_bottom {
                                for tx in cb_left..cb_right {
                                    let px = tx * 8;
                                    let py = ty * 8;
                                    for dy in 0..8 {
                                        for dx in 0..8 {
                                            let x = px + dx;
                                            let y = py + dy;
                                            if y < 240 && x < 256 {
                                                menu_framebuffer[y * 256 + x] = 0x0C0C4C;
                                            }
                                        }
                                    }
                                }
                            }
                            // Border
                            for tx in cb_left..cb_right {
                                let px = tx * 8;
                                for dx in 0..8 {
                                    let x = px + dx;
                                    let yt = cb_top * 8;
                                    let yb = cb_bottom * 8 - 1;
                                    if yt < 240 && x < 256 { menu_framebuffer[yt * 256 + x] = MENU_LIGHT_BLUE; }
                                    if yb < 240 && x < 256 { menu_framebuffer[yb * 256 + x] = MENU_LIGHT_BLUE; }
                                }
                            }
                            for ty in cb_top..cb_bottom {
                                let py = ty * 8;
                                for dy in 0..8 {
                                    let y = py + dy;
                                    if y < 240 {
                                        let xl = cb_left * 8;
                                        let xr = cb_right * 8 - 1;
                                        menu_framebuffer[y * 256 + xl] = MENU_LIGHT_BLUE;
                                        if xr < 256 { menu_framebuffer[y * 256 + xr] = MENU_LIGHT_BLUE; }
                                    }
                                }
                            }

                            draw_text_centered_8x8(&mut menu_framebuffer, "\x11 CHEATS \x11", cb_top + 1, MENU_GOLD);

                            // List cheats + ADD CODE + CLEAR ALL
                            let max_visible = 12usize; // max rows for cheat list
                            let total_items = bus.cheats.len() + 2;
                            let scroll_offset = if cheats_selected >= max_visible {
                                cheats_selected - max_visible + 1
                            } else { 0 };

                            for i in 0..max_visible.min(total_items) {
                                let item_idx = scroll_offset + i;
                                if item_idx >= total_items { break; }
                                let row = cb_top + 3 + i;
                                if row >= cb_bottom - 1 { break; }
                                let is_sel = item_idx == cheats_selected;

                                if is_sel {
                                    draw_highlight_bar(&mut menu_framebuffer, row * 8, 8, cb_left * 8 + 4, cb_right * 8 - 4, 0x3C3C8C);
                                    draw_char_8x8(&mut menu_framebuffer, '\x10', cb_left + 1, row, MENU_WHITE);
                                }

                                let color = if is_sel { MENU_WHITE } else { MENU_GRAY };
                                if item_idx < bus.cheats.len() {
                                    let status = if bus.cheats[item_idx].enabled { "ON " } else { "OFF" };
                                    let label = format!("[{}] {}", status, bus.cheats[item_idx].code_str);
                                    draw_text_8x8(&mut menu_framebuffer, &label, cb_left + 2, row, color);
                                } else if item_idx == bus.cheats.len() {
                                    draw_text_8x8(&mut menu_framebuffer, "ADD CODE...", cb_left + 2, row, color);
                                } else {
                                    draw_text_8x8(&mut menu_framebuffer, "CLEAR ALL", cb_left + 2, row, color);
                                }
                            }

                            // Cheat code text input overlay
                            if cheat_input_mode {
                                let input_y = cb_bottom - 4;
                                for dy in 0..24 {
                                    for dx in 0..160 {
                                        let x = 48 + dx;
                                        let y = input_y * 8 + dy;
                                        if y < 240 && x < 256 {
                                            menu_framebuffer[y * 256 + x] = 0x000030;
                                        }
                                    }
                                }
                                draw_text_8x8(&mut menu_framebuffer, "GAME GENIE CODE:", 7, input_y, 0xF8D878);
                                let display = if cheat_input_buffer.is_empty() { "________" } else { &cheat_input_buffer };
                                draw_text_8x8(&mut menu_framebuffer, display, 9, input_y + 1, 0xFCFCFC);
                            }

                            // Status message
                            if cheat_message_timer > 0 {
                                cheat_message_timer -= 1;
                                if let Some(ref msg) = cheat_message {
                                    draw_text_centered_8x8(&mut menu_framebuffer, msg, cb_bottom - 2, 0x44FF44);
                                }
                                if cheat_message_timer == 0 { cheat_message = None; }
                            }

                            draw_text_centered_8x8(&mut menu_framebuffer, "ESC:BACK  A:TOGGLE  BS:DEL", cb_bottom - 1, MENU_DARK_GRAY);
                        }

                        // Netplay submenu overlay
                        if netplay_submenu {
                            // Draw overlay box over the pause menu
                            let nb_left = 7;
                            let nb_right = 25;
                            let nb_top = 10;
                            let nb_bottom = 22;
                            for ty in nb_top..nb_bottom {
                                for tx in nb_left..nb_right {
                                    let px = tx * 8;
                                    let py = ty * 8;
                                    for dy in 0..8 {
                                        for dx in 0..8 {
                                            let x = px + dx;
                                            let y = py + dy;
                                            if y < 240 && x < 256 {
                                                menu_framebuffer[y * 256 + x] = 0x0C0C4C;
                                            }
                                        }
                                    }
                                }
                            }
                            // Border
                            for tx in nb_left..nb_right {
                                let px = tx * 8;
                                for dx in 0..8 {
                                    let x = px + dx;
                                    let yt = nb_top * 8;
                                    let yb = nb_bottom * 8 - 1;
                                    if yt < 240 && x < 256 { menu_framebuffer[yt * 256 + x] = MENU_LIGHT_BLUE; }
                                    if yb < 240 && x < 256 { menu_framebuffer[yb * 256 + x] = MENU_LIGHT_BLUE; }
                                }
                            }
                            for ty in nb_top..nb_bottom {
                                let py = ty * 8;
                                for dy in 0..8 {
                                    let y = py + dy;
                                    if y < 240 {
                                        let xl = nb_left * 8;
                                        let xr = nb_right * 8 - 1;
                                        menu_framebuffer[y * 256 + xl] = MENU_LIGHT_BLUE;
                                        if xr < 256 { menu_framebuffer[y * 256 + xr] = MENU_LIGHT_BLUE; }
                                    }
                                }
                            }

                            draw_text_centered_8x8(&mut menu_framebuffer, "\x11 NETPLAY \x11", nb_top + 1, MENU_GOLD);

                            // Status line
                            let status_color = if netplay.is_connected() { 0x44FF44u32 } else { MENU_GRAY };
                            draw_text_centered_8x8(&mut menu_framebuffer, netplay.status_text(), nb_top + 2, status_color);

                            let delay_str = format!("INPUT DELAY: {}", netplay.input_delay);
                            let np_items: [&str; 4] = ["HOST (PORT 7777)", "JOIN...", "DISCONNECT", &delay_str];
                            for (i, item) in np_items.iter().enumerate() {
                                let row = nb_top + 4 + i * 2;
                                let is_sel = i == netplay_selected;
                                if is_sel {
                                    draw_highlight_bar(&mut menu_framebuffer, row * 8, 8, nb_left * 8 + 4, nb_right * 8 - 4, 0x3C3C8C);
                                    draw_char_8x8(&mut menu_framebuffer, '\x10', nb_left + 1, row, MENU_WHITE);
                                }
                                let color = if is_sel { MENU_WHITE } else { MENU_GRAY };
                                draw_text_8x8(&mut menu_framebuffer, item, nb_left + 2, row, color);
                            }

                            // IP input overlay when editing
                            if netplay_ip_editing {
                                let ip_y = (nb_top + 5) * 8;
                                for dy in 0..16 {
                                    for dx in 0..128 {
                                        let x = 64 + dx;
                                        let y = ip_y + dy;
                                        if y < 240 && x < 256 {
                                            menu_framebuffer[y * 256 + x] = 0x000030;
                                        }
                                    }
                                }
                                draw_text_8x8(&mut menu_framebuffer, "IP:PORT:", 9, nb_top + 5, 0xF8D878);
                                let ip_display = if netplay_ip_input.is_empty() { "_" } else { &netplay_ip_input };
                                draw_text_8x8(&mut menu_framebuffer, ip_display, 9, nb_top + 7, 0xFCFCFC);
                            }

                            draw_text_centered_8x8(&mut menu_framebuffer, "ESC:BACK  A:SELECT", nb_bottom - 1, MENU_DARK_GRAY);
                        }

                        // Achievement submenu overlay
                        if achievement_submenu {
                            let ab_left = 4;
                            let ab_right = 28;
                            let ab_top = 5;
                            let ab_bottom = 27;
                            // Fill background
                            for ty in ab_top..ab_bottom {
                                for tx in ab_left..ab_right {
                                    let px = tx * 8;
                                    let py = ty * 8;
                                    for dy in 0..8 {
                                        for dx in 0..8 {
                                            let x = px + dx;
                                            let y = py + dy;
                                            if y < 240 && x < 256 {
                                                menu_framebuffer[y * 256 + x] = 0x0C0C3C;
                                            }
                                        }
                                    }
                                }
                            }
                            // Border
                            for tx in ab_left..ab_right {
                                let px = tx * 8;
                                for dx in 0..8 {
                                    let x = px + dx;
                                    let yt = ab_top * 8;
                                    let yb = ab_bottom * 8 - 1;
                                    if yt < 240 && x < 256 { menu_framebuffer[yt * 256 + x] = MENU_GOLD; }
                                    if yb < 240 && x < 256 { menu_framebuffer[yb * 256 + x] = MENU_GOLD; }
                                }
                            }
                            for ty in ab_top..ab_bottom {
                                let py = ty * 8;
                                for dy in 0..8 {
                                    let y = py + dy;
                                    if y < 240 {
                                        let xl = ab_left * 8;
                                        let xr = ab_right * 8 - 1;
                                        menu_framebuffer[y * 256 + xl] = MENU_GOLD;
                                        if xr < 256 { menu_framebuffer[y * 256 + xr] = MENU_GOLD; }
                                    }
                                }
                            }

                            let title = if achievement_engine.game_title.is_empty() {
                                "ACHIEVEMENTS".to_string()
                            } else {
                                format!("{}", achievement_engine.game_title)
                            };
                            draw_text_centered_8x8(&mut menu_framebuffer, &title, ab_top + 1, MENU_GOLD);

                            let stats = format!("{}/{} UNLOCKED  {}PTS", achievement_engine.unlocked_count, achievement_engine.achievements.len(), achievement_engine.total_points);
                            draw_text_centered_8x8(&mut menu_framebuffer, &stats, ab_top + 2, MENU_WHITE);

                            // List achievements (up to 16 visible)
                            let max_visible = (ab_bottom - ab_top - 4).min(achievement_engine.achievements.len());
                            for (i, ach) in achievement_engine.achievements.iter().take(max_visible).enumerate() {
                                let row = ab_top + 4 + i;
                                if row >= ab_bottom - 1 { break; }
                                let icon = if ach.unlocked { "\x0F" } else { "." };
                                let label = format!("{} {} ({})", icon, ach.title, ach.points);
                                let color = if ach.unlocked { 0x44FF44u32 } else { MENU_GRAY };
                                draw_text_8x8(&mut menu_framebuffer, &label, ab_left + 1, row, color);
                            }

                            if achievement_engine.achievements.is_empty() {
                                draw_text_centered_8x8(&mut menu_framebuffer, "NO ACHIEVEMENTS LOADED", ab_top + 6, MENU_DARK_GRAY);
                                draw_text_centered_8x8(&mut menu_framebuffer, "PLACE JSON FILES IN", ab_top + 8, MENU_DARK_GRAY);
                                draw_text_centered_8x8(&mut menu_framebuffer, "~/.nes-emulator/", ab_top + 10, MENU_DARK_GRAY);
                                draw_text_centered_8x8(&mut menu_framebuffer, "achievements/", ab_top + 11, MENU_DARK_GRAY);
                            }

                            draw_text_centered_8x8(&mut menu_framebuffer, "ESC:BACK", ab_bottom - 1, MENU_DARK_GRAY);
                        }

                        // Controls reference page overlay
                        if controls_submenu {
                            // Fill entire framebuffer for full-screen reference page
                            for i in 0..256 * 240 {
                                menu_framebuffer[i] = MENU_BG;
                            }

                            // Title
                            draw_text_centered_8x8(&mut menu_framebuffer, "\x11 CONTROLS \x11", 1, MENU_GOLD);

                            // Separator
                            let sep_y = 2 * 8 + 4;
                            for x in 8..248 {
                                if x % 4 < 2 {
                                    menu_framebuffer[sep_y * 256 + x] = MENU_DARK_GRAY;
                                }
                            }

                            // --- Player 1 & 2 Keyboard ---
                            draw_text_8x8(&mut menu_framebuffer, "P1 KEYBOARD", 2, 3, MENU_GOLD);
                            draw_text_8x8(&mut menu_framebuffer, "P2 KEYBOARD", 18, 3, MENU_GOLD);

                            let kb1 = &config.input_bindings.keyboard_p1;
                            let kb2 = &config.input_bindings.keyboard_p2;
                            let kb_rows: [(&str, &str, &str); 10] = [
                                ("Up",     &kb1.up,      &kb2.up),
                                ("Down",   &kb1.down,    &kb2.down),
                                ("Left",   &kb1.left,    &kb2.left),
                                ("Right",  &kb1.right,   &kb2.right),
                                ("A",      &kb1.a,       &kb2.a),
                                ("B",      &kb1.b,       &kb2.b),
                                ("Start",  &kb1.start,   &kb2.start),
                                ("Select", &kb1.select,  &kb2.select),
                                ("TrboA",  &kb1.turbo_a, &kb2.turbo_a),
                                ("TrboB",  &kb1.turbo_b, &kb2.turbo_b),
                            ];
                            for (i, (label, v1, v2)) in kb_rows.iter().enumerate() {
                                let y = 4 + i;
                                draw_text_8x8(&mut menu_framebuffer, label, 2, y, MENU_GRAY);
                                draw_text_8x8(&mut menu_framebuffer, v1, 9, y, MENU_WHITE);
                                draw_text_8x8(&mut menu_framebuffer, label, 18, y, MENU_GRAY);
                                draw_text_8x8(&mut menu_framebuffer, v2, 25, y, MENU_WHITE);
                            }

                            // --- Controller P1 & P2 ---
                            draw_text_8x8(&mut menu_framebuffer, "CONTROLLER P1", 2, 15, MENU_GOLD);
                            draw_text_8x8(&mut menu_framebuffer, "CONTROLLER P2", 18, 15, MENU_GOLD);

                            let ct1 = &config.input_bindings.controller_p1;
                            let ct2 = &config.input_bindings.controller_p2;
                            let ct_rows: [(&str, &str, &str); 6] = [
                                ("A",      &ct1.a,       &ct2.a),
                                ("B",      &ct1.b,       &ct2.b),
                                ("TrboA",  &ct1.turbo_a, &ct2.turbo_a),
                                ("TrboB",  &ct1.turbo_b, &ct2.turbo_b),
                                ("Start",  &ct1.start,   &ct2.start),
                                ("Select", &ct1.select,  &ct2.select),
                            ];
                            for (i, (label, v1, v2)) in ct_rows.iter().enumerate() {
                                let y = 16 + i;
                                draw_text_8x8(&mut menu_framebuffer, label, 2, y, MENU_GRAY);
                                draw_text_8x8(&mut menu_framebuffer, v1, 9, y, MENU_WHITE);
                                draw_text_8x8(&mut menu_framebuffer, label, 18, y, MENU_GRAY);
                                draw_text_8x8(&mut menu_framebuffer, v2, 25, y, MENU_WHITE);
                            }

                            // --- System Shortcuts ---
                            draw_text_centered_8x8(&mut menu_framebuffer, "SYSTEM SHORTCUTS", 23, MENU_GOLD);

                            draw_text_8x8(&mut menu_framebuffer, "Pause",  2, 24, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Escape", 9, 24, MENU_WHITE);
                            draw_text_8x8(&mut menu_framebuffer, "Save",   18, 24, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "F5",     25, 24, MENU_WHITE);

                            draw_text_8x8(&mut menu_framebuffer, "Load",   2, 25, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "F9",     9, 25, MENU_WHITE);
                            draw_text_8x8(&mut menu_framebuffer, "Rewind", 18, 25, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Bksp",   25, 25, MENU_WHITE);

                            draw_text_8x8(&mut menu_framebuffer, "FF",     2, 26, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Tab",    9, 26, MENU_WHITE);
                            draw_text_8x8(&mut menu_framebuffer, "Reset",  18, 26, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Ctrl+R", 25, 26, MENU_WHITE);

                            draw_text_8x8(&mut menu_framebuffer, "Record", 2, 27, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Shft+R", 9, 27, MENU_WHITE);
                            draw_text_8x8(&mut menu_framebuffer, "Play",   18, 27, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Shft+P", 25, 27, MENU_WHITE);

                            draw_text_centered_8x8(&mut menu_framebuffer, "ESC TO GO BACK", 29, MENU_DARK_GRAY);
                        }
                        
                        // Now pass through CRT filter (same as menu rendering)
                        let dt = if barrel_distortion { &distortion_table } else { &flat_distortion_table };
                        if crt_enabled {
                            crt_filter(&menu_framebuffer, &mut crt_buffer, &vignette_table, dt, &config.crt_config, &mask_table);
                            // Phosphor bloom — bright pixels glow into neighbors
                            apply_phosphor_bloom(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                            apply_scanline_glow(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                            // Apply chromatic aberration to crt_buffer (screen area only)
                            if glass_intensity > 30 {
                                ca_temp.copy_from_slice(&crt_buffer[..SCREEN_W * SCREEN_H]);
                                apply_chromatic_aberration(&mut crt_buffer, &ca_temp, &ca_table, SCREEN_W, SCREEN_H);
                            }
                        } else {
                            scale_simple(&menu_framebuffer, &mut crt_buffer);
                        }
                        composite_screen_fast(&mut composite_buffer, &crt_buffer, WINDOW_WIDTH);
                        if crt_enabled {
                            apply_screen_glare(&mut composite_buffer, &glare_table, &glass_thickness_table, WINDOW_WIDTH, glass_intensity);
                            // Internal ghost reflection from thick CRT glass
                            if glass_intensity > 20 {
                                for y in 0..SCREEN_H {
                                    let row_start = (y + SCREEN_Y) * WINDOW_WIDTH + SCREEN_X;
                                    ghost_buffer[row_start..row_start + SCREEN_W]
                                        .copy_from_slice(&composite_buffer[row_start..row_start + SCREEN_W]);
                                }
                                apply_internal_ghost(&mut composite_buffer, &ghost_buffer, &ghost_alpha_table, WINDOW_WIDTH);
                            }
                        }

                        // Render save state thumbnail in pause menu (composite buffer)
                        // Position thumbnail to the right of the pause menu box
                        let thumb_scale = 2usize;
                        let thumb_w = 64 * thumb_scale;
                        let thumb_h = 60 * thumb_scale;
                        // Place thumbnail right of center, aligned with save/load items
                        // The NES screen maps to SCREEN_X..SCREEN_X+SCREEN_W in composite
                        // Menu box right edge is at tile 26 = pixel 208 in NES coords
                        // Scale factor from NES to screen: SCREEN_W / 256
                        let scale_x = SCREEN_W as f32 / 256.0;
                        let scale_y = SCREEN_H as f32 / 240.0;
                        let thumb_cx = SCREEN_X + ((26 * 8 + 8) as f32 * scale_x) as usize;
                        let thumb_cy = SCREEN_Y + ((12 * 8) as f32 * scale_y) as usize;
                        let slot_idx = (current_save_slot as usize).saturating_sub(1).min(3);
                        if let Some(ref thumb_data) = thumbnail_cache[slot_idx] {
                            // Render thumbnail upscaled 2×
                            for ty in 0..60usize {
                                for tx in 0..64usize {
                                    let src = (ty * 64 + tx) * 3;
                                    if src + 2 >= thumb_data.len() { continue; }
                                    let r = thumb_data[src] as u32;
                                    let g = thumb_data[src + 1] as u32;
                                    let b = thumb_data[src + 2] as u32;
                                    let color = (r << 16) | (g << 8) | b;
                                    for sy in 0..thumb_scale {
                                        for sx in 0..thumb_scale {
                                            let px = thumb_cx + tx * thumb_scale + sx;
                                            let py = thumb_cy + ty * thumb_scale + sy;
                                            if px < WINDOW_WIDTH && py < WINDOW_HEIGHT {
                                                let idx = py * WINDOW_WIDTH + px;
                                                if idx < composite_buffer.len() {
                                                    composite_buffer[idx] = color;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // Border around thumbnail
                            for dx in 0..thumb_w + 2 {
                                let bx = thumb_cx + dx - 1;
                                let by_top = thumb_cy.saturating_sub(1);
                                let by_bot = thumb_cy + thumb_h;
                                if bx < WINDOW_WIDTH {
                                    if by_top < WINDOW_HEIGHT {
                                        composite_buffer[by_top * WINDOW_WIDTH + bx] = 0x4444AA;
                                    }
                                    if by_bot < WINDOW_HEIGHT {
                                        composite_buffer[by_bot * WINDOW_WIDTH + bx] = 0x4444AA;
                                    }
                                }
                            }
                            for dy in 0..thumb_h {
                                let by = thumb_cy + dy;
                                let bx_l = thumb_cx.saturating_sub(1);
                                let bx_r = thumb_cx + thumb_w;
                                if by < WINDOW_HEIGHT {
                                    if bx_l < WINDOW_WIDTH {
                                        composite_buffer[by * WINDOW_WIDTH + bx_l] = 0x4444AA;
                                    }
                                    if bx_r < WINDOW_WIDTH {
                                        composite_buffer[by * WINDOW_WIDTH + bx_r] = 0x4444AA;
                                    }
                                }
                            }
                            // Slot label above thumbnail
                            let label = format!("SLOT {}", current_save_slot);
                            draw_text(&mut composite_buffer, &label, thumb_cx, thumb_cy.saturating_sub(10), 0xF8D878, WINDOW_WIDTH);
                        } else {
                            // Empty slot: dark background with "EMPTY" text
                            for dy in 0..thumb_h {
                                for dx in 0..thumb_w {
                                    let px = thumb_cx + dx;
                                    let py = thumb_cy + dy;
                                    if px < WINDOW_WIDTH && py < WINDOW_HEIGHT {
                                        let idx = py * WINDOW_WIDTH + px;
                                        if idx < composite_buffer.len() {
                                            composite_buffer[idx] = 0x181830;
                                        }
                                    }
                                }
                            }
                            // Border
                            for dx in 0..thumb_w + 2 {
                                let bx = thumb_cx + dx - 1;
                                let by_top = thumb_cy.saturating_sub(1);
                                let by_bot = thumb_cy + thumb_h;
                                if bx < WINDOW_WIDTH {
                                    if by_top < WINDOW_HEIGHT {
                                        composite_buffer[by_top * WINDOW_WIDTH + bx] = 0x333366;
                                    }
                                    if by_bot < WINDOW_HEIGHT {
                                        composite_buffer[by_bot * WINDOW_WIDTH + bx] = 0x333366;
                                    }
                                }
                            }
                            for dy in 0..thumb_h {
                                let by = thumb_cy + dy;
                                let bx_l = thumb_cx.saturating_sub(1);
                                let bx_r = thumb_cx + thumb_w;
                                if by < WINDOW_HEIGHT {
                                    if bx_l < WINDOW_WIDTH {
                                        composite_buffer[by * WINDOW_WIDTH + bx_l] = 0x333366;
                                    }
                                    if bx_r < WINDOW_WIDTH {
                                        composite_buffer[by * WINDOW_WIDTH + bx_r] = 0x333366;
                                    }
                                }
                            }
                            let label = format!("SLOT {}", current_save_slot);
                            draw_text(&mut composite_buffer, &label, thumb_cx, thumb_cy.saturating_sub(10), 0xF8D878, WINDOW_WIDTH);
                            let empty_x = thumb_cx + (thumb_w - 5 * 4) / 2;
                            let empty_y = thumb_cy + (thumb_h - 5) / 2;
                            draw_text(&mut composite_buffer, "EMPTY", empty_x, empty_y, 0x666688, WINDOW_WIDTH);
                        }
                    } else if quick_overlay {
                        // Quick overlay rendering
                        // Darken the CRT output by 50%
                        for i in 0..menu_framebuffer.len().min(bus.ppu.frame_data.len()) {
                            let p = bus.ppu.frame_data[i];
                            let r = ((p >> 16) & 0xFF) >> 1;
                            let g = ((p >> 8) & 0xFF) >> 1;
                            let b = (p & 0xFF) >> 1;
                            menu_framebuffer[i] = (r << 16) | (g << 8) | b;
                        }

                        // Draw centered overlay box (compact: 20 tiles wide × 12 tiles tall)
                        let box_left: usize = 6;
                        let box_right: usize = 26;
                        let box_top: usize = 8;
                        let box_bottom: usize = 22;

                        // Fill box background
                        for ty in box_top..box_bottom {
                            for tx in box_left..box_right {
                                let px = tx * 8;
                                let py = ty * 8;
                                for dy in 0..8 {
                                    for dx in 0..8 {
                                        let x = px + dx;
                                        let y = py + dy;
                                        if y < 240 && x < 256 {
                                            menu_framebuffer[y * 256 + x] = 0x0A0A1A;
                                        }
                                    }
                                }
                            }
                        }

                        // Border
                        for tx in box_left..box_right {
                            let px = tx * 8;
                            for dx in 0..8 {
                                let x = px + dx;
                                let yt = box_top * 8;
                                let yb = box_bottom * 8 - 1;
                                if yt < 240 && x < 256 {
                                    menu_framebuffer[yt * 256 + x] = 0x4080C0;
                                    if yt + 1 < 240 { menu_framebuffer[(yt + 1) * 256 + x] = 0x4080C0; }
                                }
                                if yb < 240 && x < 256 {
                                    menu_framebuffer[yb * 256 + x] = 0x4080C0;
                                    if yb > 0 { menu_framebuffer[(yb - 1) * 256 + x] = 0x4080C0; }
                                }
                            }
                        }
                        for ty in box_top..box_bottom {
                            let py = ty * 8;
                            for dy in 0..8 {
                                let y = py + dy;
                                if y < 240 {
                                    let xl = box_left * 8;
                                    let xr = box_right * 8 - 1;
                                    if xl < 256 { menu_framebuffer[y * 256 + xl] = 0x4080C0; menu_framebuffer[y * 256 + xl + 1] = 0x4080C0; }
                                    if xr < 256 { menu_framebuffer[y * 256 + xr] = 0x4080C0; if xr > 0 { menu_framebuffer[y * 256 + xr - 1] = 0x4080C0; } }
                                }
                            }
                        }

                        // Title
                        draw_text_centered_8x8(&mut menu_framebuffer, "\x11 QUICK MENU \x11", box_top + 1, MENU_GOLD);

                        // Separator
                        let sep_y = (box_top + 2) * 8 + 4;
                        for x in (box_left * 8 + 8)..(box_right * 8 - 8) {
                            if x % 4 < 2 && sep_y < 240 {
                                menu_framebuffer[sep_y * 256 + x] = 0x404040;
                            }
                        }

                        // Menu items
                        let fav_label = if is_favorite(&config, &current_rom_path) {
                            "\x11 REMOVE FAVORITE"
                        } else {
                            "\x11 ADD FAVORITE"
                        };
                        let save_label = format!("SAVE STATE  [SLOT {}]", current_save_slot);
                        let load_label = format!("LOAD STATE  [SLOT {}]", current_save_slot);
                        let items: [&str; 6] = [
                            "\x10 RESUME",
                            &save_label,
                            &load_label,
                            fav_label,
                            "MORE OPTIONS...",
                            "RETURN TO MENU",
                        ];

                        for (i, item) in items.iter().enumerate() {
                            let row = box_top + 3 + i;
                            let is_selected = i == quick_overlay_selected;

                            if is_selected {
                                draw_highlight_bar(&mut menu_framebuffer, row * 8, 8, box_left * 8 + 4, box_right * 8 - 4, 0x2A2A6A);
                            }

                            let color = if is_selected { 0xFFFFFF } else { 0xA0A0A0 };
                            if is_selected {
                                draw_char_8x8(&mut menu_framebuffer, '\x10', box_left + 1, row, 0xFFFFFF);
                            }
                            draw_text_8x8(&mut menu_framebuffer, item, box_left + 2, row, color);
                        }

                        // Hint at bottom
                        draw_text_centered_8x8(&mut menu_framebuffer, "B:CLOSE  L/R:SLOT", box_bottom - 1, 0x606060);

                        // Pass through CRT filter (same as pause menu rendering)
                        let dt = if barrel_distortion { &distortion_table } else { &flat_distortion_table };
                        if crt_enabled {
                            crt_filter(&menu_framebuffer, &mut crt_buffer, &vignette_table, dt, &config.crt_config, &mask_table);
                            apply_phosphor_bloom(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                            apply_scanline_glow(&mut crt_buffer, SCREEN_W, SCREEN_H, config.crt_config.phosphor_warmth as u32);
                            if glass_intensity > 30 {
                                ca_temp.copy_from_slice(&crt_buffer[..SCREEN_W * SCREEN_H]);
                                apply_chromatic_aberration(&mut crt_buffer, &ca_temp, &ca_table, SCREEN_W, SCREEN_H);
                            }
                        } else {
                            scale_simple(&menu_framebuffer, &mut crt_buffer);
                        }
                        composite_screen_fast(&mut composite_buffer, &crt_buffer, WINDOW_WIDTH);
                        if crt_enabled {
                            apply_screen_glare(&mut composite_buffer, &glare_table, &glass_thickness_table, WINDOW_WIDTH, glass_intensity);
                            if glass_intensity > 20 {
                                for y in 0..SCREEN_H {
                                    let row_start = (y + SCREEN_Y) * WINDOW_WIDTH + SCREEN_X;
                                    ghost_buffer[row_start..row_start + SCREEN_W]
                                        .copy_from_slice(&composite_buffer[row_start..row_start + SCREEN_W]);
                                }
                                apply_internal_ghost(&mut composite_buffer, &ghost_buffer, &ghost_alpha_table, WINDOW_WIDTH);
                            }
                        }
                    }

                    window
                        .update_with_buffer(&composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
                        .expect("Failed to update window");

                    // Mouse click handling for console interactions (only when not paused)
                    if !paused {
                        let _mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
                    }
                } else {
                    next_state = Some(EmulatorState::Menu(MenuState::new()));
                    window_title = format!("OxideNES v{}", env!("OXIDENES_VERSION"));
                    window.set_title(&window_title);
                }
            }
        }

        // Apply deferred state transitions
        if let Some(new_state) = next_state {
            emulator_state = new_state;
        }
    }
}


/// Convert analog stick to NES D-pad with circular deadzone, cardinal snapping, and hysteresis.
/// Returns (up, down, left, right) as booleans.
#[inline]
fn stick_to_dpad(
    stick_x: f32,
    stick_y: f32,
    deadzone: f32,
    prev_state: &mut StickState,
) -> (bool, bool, bool, bool) {
    let magnitude = (stick_x * stick_x + stick_y * stick_y).sqrt();

    // Hysteresis: use lower threshold for release to prevent jitter
    let active_deadzone = if prev_state.any_active() {
        (deadzone * 0.75).max(0.05)
    } else {
        deadzone
    };

    if magnitude < active_deadzone {
        prev_state.clear();
        return (false, false, false, false);
    }

    // Normalize to unit circle
    let nx = stick_x / magnitude;
    let ny = stick_y / magnitude;

    // Angle in degrees (0° = right, 90° = up, counter-clockwise)
    let angle = ny.atan2(nx).to_degrees();
    let angle = if angle < 0.0 { angle + 360.0 } else { angle };

    // Push strength: 0.0 at deadzone edge, 1.0 at full tilt
    let push_strength = ((magnitude - active_deadzone) / (1.0 - active_deadzone)).min(1.0);

    // Cardinal snapping: 35° half-angle gives 70° pure-cardinal zones
    // Diagonals only allowed in the remaining 20° windows at >70% push
    let cardinal_half_angle = 35.0_f32;
    let diagonal_min_strength = 0.70_f32;

    let mut up = false;
    let mut down = false;
    let mut left = false;
    let mut right = false;

    let angle_dist = |target: f32| -> f32 {
        let d = (angle - target).abs();
        if d > 180.0 { 360.0 - d } else { d }
    };

    let dist_right = angle_dist(0.0);
    let dist_up = angle_dist(90.0);
    let dist_left = angle_dist(180.0);
    let dist_down = angle_dist(270.0);

    let min_dist = dist_right.min(dist_up).min(dist_left).min(dist_down);

    if min_dist <= cardinal_half_angle {
        // Pure cardinal zone
        if min_dist == dist_right { right = true; }
        else if min_dist == dist_up { up = true; }
        else if min_dist == dist_left { left = true; }
        else { down = true; }
    } else if push_strength >= diagonal_min_strength {
        // Diagonal zone with sufficient push
        if angle > 0.0 && angle < 90.0 { right = true; up = true; }
        else if angle > 90.0 && angle < 180.0 { left = true; up = true; }
        else if angle > 180.0 && angle < 270.0 { left = true; down = true; }
        else { right = true; down = true; }
    } else {
        // Diagonal zone but not pushed hard enough — snap to nearest cardinal
        if min_dist == dist_right { right = true; }
        else if min_dist == dist_up { up = true; }
        else if min_dist == dist_left { left = true; }
        else { down = true; }
    }

    // SOCD cleaning: prevent simultaneous opposite directions
    if up && down { up = false; down = false; }
    if left && right { left = false; right = false; }

    prev_state.up = up;
    prev_state.down = down;
    prev_state.left = left;
    prev_state.right = right;

    (up, down, left, right)
}

fn handle_input(window: &Window, bus: &mut Bus, gilrs: &mut Option<Gilrs>, frame_counter: u32, input_bindings: &InputBindings, stick_state_p1: &mut StickState, stick_state_p2: &mut StickState) -> (bool, bool, bool, bool) {
    let keys = window.get_keys();
    let turbo_active = (frame_counter / 2) % 2 == 0; // ~15Hz: ON 2 frames, OFF 2 frames

    // Player 1 - Keyboard
    let kb1 = &input_bindings.keyboard_p1;
    let p1_key_up = string_to_key(&kb1.up);
    let p1_key_down = string_to_key(&kb1.down);
    let p1_key_left = string_to_key(&kb1.left);
    let p1_key_right = string_to_key(&kb1.right);
    let p1_key_a = string_to_key(&kb1.a);
    let p1_key_b = string_to_key(&kb1.b);
    let p1_key_start = string_to_key(&kb1.start);
    let p1_key_select = string_to_key(&kb1.select);
    let p1_key_turbo_a = string_to_key(&kb1.turbo_a);
    let p1_key_turbo_b = string_to_key(&kb1.turbo_b);

    let mut p1_up = p1_key_up.map_or(false, |k| keys.contains(&k));
    let mut p1_down = p1_key_down.map_or(false, |k| keys.contains(&k));
    let mut p1_left = p1_key_left.map_or(false, |k| keys.contains(&k));
    let mut p1_right = p1_key_right.map_or(false, |k| keys.contains(&k));
    let mut p1_a = p1_key_a.map_or(false, |k| keys.contains(&k));
    let mut p1_b = p1_key_b.map_or(false, |k| keys.contains(&k));
    let mut p1_start = p1_key_start.map_or(false, |k| keys.contains(&k));
    let mut p1_select = p1_key_select.map_or(false, |k| keys.contains(&k));
    let mut l_trigger = false;
    let mut r_trigger = false;

    // P1 turbo buttons
    if p1_key_turbo_a.map_or(false, |k| keys.contains(&k)) && turbo_active {
        p1_a = true;
    }
    if p1_key_turbo_b.map_or(false, |k| keys.contains(&k)) && turbo_active {
        p1_b = true;
    }

    // Player 2 - Keyboard
    let kb2 = &input_bindings.keyboard_p2;
    let p2_key_up = string_to_key(&kb2.up);
    let p2_key_down = string_to_key(&kb2.down);
    let p2_key_left = string_to_key(&kb2.left);
    let p2_key_right = string_to_key(&kb2.right);
    let p2_key_a = string_to_key(&kb2.a);
    let p2_key_b = string_to_key(&kb2.b);
    let p2_key_start = string_to_key(&kb2.start);
    let p2_key_select = string_to_key(&kb2.select);
    let p2_key_turbo_a = string_to_key(&kb2.turbo_a);
    let p2_key_turbo_b = string_to_key(&kb2.turbo_b);

    let mut p2_up = p2_key_up.map_or(false, |k| keys.contains(&k));
    let mut p2_down = p2_key_down.map_or(false, |k| keys.contains(&k));
    let mut p2_left = p2_key_left.map_or(false, |k| keys.contains(&k));
    let mut p2_right = p2_key_right.map_or(false, |k| keys.contains(&k));
    let mut p2_a = p2_key_a.map_or(false, |k| keys.contains(&k));
    let mut p2_b = p2_key_b.map_or(false, |k| keys.contains(&k));
    let mut p2_start = p2_key_start.map_or(false, |k| keys.contains(&k));
    let mut p2_select = p2_key_select.map_or(false, |k| keys.contains(&k));

    // P2 turbo buttons
    if p2_key_turbo_a.map_or(false, |k| keys.contains(&k)) && turbo_active {
        p2_a = true;
    }
    if p2_key_turbo_b.map_or(false, |k| keys.contains(&k)) && turbo_active {
        p2_b = true;
    }

    // Controllers - Poll gamepad events and read state
    if let Some(ref mut g) = gilrs {
        // Process pending events (required by gilrs)
        while let Some(_event) = g.next_event() {}

        // Get all connected gamepads
        let mut gp_iter = g.gamepads().filter(|(_, gp)| gp.is_connected());
        
        // Player 1 controller
        if let Some((_, gamepad)) = gp_iter.next() {
            let ctrl1 = &input_bindings.controller_p1;
            
            // D-pad buttons
            p1_up |= gamepad.is_pressed(Button::DPadUp);
            p1_down |= gamepad.is_pressed(Button::DPadDown);
            p1_left |= gamepad.is_pressed(Button::DPadLeft);
            p1_right |= gamepad.is_pressed(Button::DPadRight);

            // Left analog stick (circular deadzone + cardinal snapping)
            let stick_x = gamepad.value(Axis::LeftStickX);
            let stick_y = gamepad.value(Axis::LeftStickY);
            let (s_up, s_down, s_left, s_right) = stick_to_dpad(
                stick_x, stick_y, ctrl1.deadzone, stick_state_p1
            );
            p1_up    |= s_up;
            p1_down  |= s_down;
            p1_left  |= s_left;
            p1_right |= s_right;

            // Face buttons - configurable
            if let Some(btn) = string_to_gilrs_button(&ctrl1.a) {
                p1_a |= gamepad.is_pressed(btn);
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl1.b) {
                p1_b |= gamepad.is_pressed(btn);
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl1.turbo_a) {
                if gamepad.is_pressed(btn) && turbo_active {
                    p1_a = true;
                }
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl1.turbo_b) {
                if gamepad.is_pressed(btn) && turbo_active {
                    p1_b = true;
                }
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl1.start) {
                p1_start |= gamepad.is_pressed(btn);
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl1.select) {
                p1_select |= gamepad.is_pressed(btn);
            }
            l_trigger |= gamepad.is_pressed(Button::LeftTrigger) || gamepad.is_pressed(Button::LeftTrigger2);
            r_trigger |= gamepad.is_pressed(Button::RightTrigger) || gamepad.is_pressed(Button::RightTrigger2);
        }

        // Player 2 controller
        if let Some((_, gamepad)) = gp_iter.next() {
            let ctrl2 = &input_bindings.controller_p2;
            
            // D-pad buttons
            p2_up |= gamepad.is_pressed(Button::DPadUp);
            p2_down |= gamepad.is_pressed(Button::DPadDown);
            p2_left |= gamepad.is_pressed(Button::DPadLeft);
            p2_right |= gamepad.is_pressed(Button::DPadRight);

            // Left analog stick (circular deadzone + cardinal snapping)
            let stick_x = gamepad.value(Axis::LeftStickX);
            let stick_y = gamepad.value(Axis::LeftStickY);
            let (s_up, s_down, s_left, s_right) = stick_to_dpad(
                stick_x, stick_y, ctrl2.deadzone, stick_state_p2
            );
            p2_up    |= s_up;
            p2_down  |= s_down;
            p2_left  |= s_left;
            p2_right |= s_right;

            // Face buttons - configurable
            if let Some(btn) = string_to_gilrs_button(&ctrl2.a) {
                p2_a |= gamepad.is_pressed(btn);
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl2.b) {
                p2_b |= gamepad.is_pressed(btn);
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl2.turbo_a) {
                if gamepad.is_pressed(btn) && turbo_active {
                    p2_a = true;
                }
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl2.turbo_b) {
                if gamepad.is_pressed(btn) && turbo_active {
                    p2_b = true;
                }
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl2.start) {
                p2_start |= gamepad.is_pressed(btn);
            }
            if let Some(btn) = string_to_gilrs_button(&ctrl2.select) {
                p2_select |= gamepad.is_pressed(btn);
            }
        }
    }

    // Apply all input to joypads
    bus.joypad1.set_button_pressed(JoypadButton::A, p1_a);
    bus.joypad1.set_button_pressed(JoypadButton::B, p1_b);
    bus.joypad1.set_button_pressed(JoypadButton::Select, p1_select);
    bus.joypad1.set_button_pressed(JoypadButton::Start, p1_start);
    bus.joypad1.set_button_pressed(JoypadButton::Up, p1_up);
    bus.joypad1.set_button_pressed(JoypadButton::Down, p1_down);
    bus.joypad1.set_button_pressed(JoypadButton::Left, p1_left);
    bus.joypad1.set_button_pressed(JoypadButton::Right, p1_right);

    bus.joypad2.set_button_pressed(JoypadButton::A, p2_a);
    bus.joypad2.set_button_pressed(JoypadButton::B, p2_b);
    bus.joypad2.set_button_pressed(JoypadButton::Select, p2_select);
    bus.joypad2.set_button_pressed(JoypadButton::Start, p2_start);
    bus.joypad2.set_button_pressed(JoypadButton::Up, p2_up);
    bus.joypad2.set_button_pressed(JoypadButton::Down, p2_down);
    bus.joypad2.set_button_pressed(JoypadButton::Left, p2_left);
    bus.joypad2.set_button_pressed(JoypadButton::Right, p2_right);

    (p1_start, p1_select, l_trigger, r_trigger)
}

// Optimized: Inline all per-pixel color operations
#[inline(always)]
fn blend_bilinear_rgb(p00: u32, p10: u32, p01: u32, p11: u32, frac_x: u32, frac_y: u32) -> (u32, u32, u32) {
    let inv_fx = 256 - frac_x;
    let inv_fy = 256 - frac_y;
    
    // Process all channels in parallel using bit shifts
    let r = ((p00 >> 16) & 0xFF) * inv_fx * inv_fy
            + ((p10 >> 16) & 0xFF) * frac_x * inv_fy
            + ((p01 >> 16) & 0xFF) * inv_fx * frac_y
            + ((p11 >> 16) & 0xFF) * frac_x * frac_y >> 16;
    
    let g = ((p00 >> 8) & 0xFF) * inv_fx * inv_fy
            + ((p10 >> 8) & 0xFF) * frac_x * inv_fy
            + ((p01 >> 8) & 0xFF) * inv_fx * frac_y
            + ((p11 >> 8) & 0xFF) * frac_x * frac_y >> 16;
    
    let b = (p00 & 0xFF) * inv_fx * inv_fy
            + (p10 & 0xFF) * frac_x * inv_fy
            + (p01 & 0xFF) * inv_fx * frac_y
            + (p11 & 0xFF) * frac_x * frac_y >> 16;
    
    (r, g, b)
}

#[inline(always)]
fn apply_blur_3tap(r: u32, g: u32, b: u32, left: u32, right: u32, blur_center: u32, blur_side: u32) -> (u32, u32, u32) {
    let r = (r * blur_center + ((left >> 16) & 0xFF) * blur_side + ((right >> 16) & 0xFF) * blur_side) >> 8;
    let g = (g * blur_center + ((left >> 8) & 0xFF) * blur_side + ((right >> 8) & 0xFF) * blur_side) >> 8;
    let b = (b * blur_center + (left & 0xFF) * blur_side + (right & 0xFF) * blur_side) >> 8;
    (r, g, b)
}

/// Integer square root for const context
const fn isqrt_const(n: u32) -> u32 {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// CRT gamma lookup table — precomputed for γ2.2 (inverse gamma for display)
/// Maps input luminance to CRT output: out = (in/255)^(1/2.2) * 255
/// Brightens midtones slightly to simulate how CRT electron guns respond to voltage.
/// Using sqrt approximation: (1/2.2) ≈ 0.4545 ≈ 90% sqrt + 10% linear
const GAMMA_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        let sq = isqrt_const(i as u32 * 255);
        let lin = i as u32;
        let val = (sq * 230 + lin * 26) / 256; // ~90% sqrt curve, 10% linear
        table[i] = if val > 255 { 255 } else { val as u8 };
        i += 1;
    }
    table
};

#[inline(always)]
fn apply_phosphor(r: u32, g: u32, b: u32, pr_mul: u32, pg_mul: u32, pb_mul: u32) -> (u32, u32, u32) {
    // Brightness-dependent warmth: bright pixels get full warmth shift,
    // dark pixels stay neutral (mimics phosphor heating on consumer TVs)
    let brightness = ((r + g + b) * 85) >> 8; // fast /3, range 0-255
    // Interpolate: at brightness=0 use 256 (neutral), at 255 use full pr/pg/pb_mul
    // delta = (mul - 256), which can be negative for green/blue channels
    // mul_actual = 256 + delta * brightness / 255
    let pr_delta = pr_mul as i32 - 256;
    let pg_delta = pg_mul as i32 - 256;
    let pb_delta = pb_mul as i32 - 256;
    let br = brightness as i32;
    let pr = (256 + (pr_delta * br / 255)) as u32;
    let pg = (256 + (pg_delta * br / 255)) as u32;
    let pb = (256 + (pb_delta * br / 255)) as u32;
    ((r * pr) >> 8, (g * pg) >> 8, (b * pb) >> 8)
}

#[inline(always)]
fn apply_scanline_vignette(r: u32, g: u32, b: u32, scan_mul: u32, vig: u32) -> (u32, u32, u32) {
    // Combine scanline and vignette in one multiplication to reduce ops
    let combined = (scan_mul * vig) >> 8;
    ((r * combined) >> 8, (g * combined) >> 8, (b * combined) >> 8)
}

#[inline(always)]
fn apply_mask(r: u32, g: u32, b: u32, mr: u8, mg: u8, mb: u8, intensity: u32) -> (u32, u32, u32) {
    // intensity 0 = no mask effect, 100 = full mask effect
    // Lerp: result = original * (100 - intensity)/100 + masked * intensity/100
    // / 255 approx: (x * 0x8081) >> 23  — exact for 0..=65025
    let masked_r = ((r * mr as u32) * 0x8081) >> 23;
    let masked_g = ((g * mg as u32) * 0x8081) >> 23;
    let masked_b = ((b * mb as u32) * 0x8081) >> 23;
    let inv = 100 - intensity;
    // / 100 approx: (x * 1311) >> 17  — accurate for 0..=131071
    (
        ((r * inv + masked_r * intensity) * 1311) >> 17,
        ((g * inv + masked_g * intensity) * 1311) >> 17,
        ((b * inv + masked_b * intensity) * 1311) >> 17,
    )
}

#[inline(always)]
fn pack_rgb(r: u32, g: u32, b: u32) -> u32 {
    (r.min(255) << 16) | (g.min(255) << 8) | b.min(255)
}

/// Phosphor bloom: horizontal glow for bright pixels.
/// Forward + backward pass — no allocations, cache-friendly.
#[inline]
fn apply_phosphor_bloom(buffer: &mut [u32], width: usize, height: usize, bloom_strength: u32) {
    if bloom_strength == 0 { return; }

    let threshold: u32 = 180;
    let bleed = (bloom_strength * 16 / 100).min(15); // 0-15 range
    if bleed == 0 { return; }

    // Single horizontal forward pass only — fast, cache-friendly, good enough visually
    for y in 0..height {
        let row = y * width;
        let mut carry_r: u32 = 0;
        let mut carry_g: u32 = 0;
        let mut carry_b: u32 = 0;

        for x in 0..width {
            let idx = row + x;
            let pixel = unsafe { *buffer.get_unchecked(idx) };
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            // Apply carry from previous bright pixel
            if carry_r | carry_g | carry_b != 0 {
                let nr = (r + carry_r).min(255);
                let ng = (g + carry_g).min(255);
                let nb = (b + carry_b).min(255);
                unsafe { *buffer.get_unchecked_mut(idx) = (nr << 16) | (ng << 8) | nb; }
                // Decay carry quickly
                carry_r >>= 1;
                carry_g >>= 1;
                carry_b >>= 1;
            }

            // Bright pixels generate new carry
            let brightness = ((r + g + b) * 85) >> 8; // fast /3 approximation
            if brightness > threshold {
                let excess = brightness - threshold;
                carry_r = (r * excess * bleed) >> 16;
                carry_g = (g * excess * bleed) >> 16;
                carry_b = (b * excess * bleed) >> 16;
            }
        }
    }

    // Backward pass (right→left) — makes glow symmetric
    for y in 0..height {
        let row = y * width;
        let mut carry_r: u32 = 0;
        let mut carry_g: u32 = 0;
        let mut carry_b: u32 = 0;

        for x in (0..width).rev() {
            let idx = row + x;
            let pixel = unsafe { *buffer.get_unchecked(idx) };
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            if carry_r | carry_g | carry_b != 0 {
                let nr = (r + carry_r).min(255);
                let ng = (g + carry_g).min(255);
                let nb = (b + carry_b).min(255);
                unsafe { *buffer.get_unchecked_mut(idx) = (nr << 16) | (ng << 8) | nb; }
                carry_r >>= 1;
                carry_g >>= 1;
                carry_b >>= 1;
            }

            let brightness = ((r + g + b) * 85) >> 8;
            if brightness > threshold {
                let excess = brightness - threshold;
                carry_r = (r * excess * bleed) >> 16;
                carry_g = (g * excess * bleed) >> 16;
                carry_b = (b * excess * bleed) >> 16;
            }
        }
    }
}

/// Inter-scanline phosphor glow: bright rows bleed into dark scanline gap rows.
/// Only processes gap rows (every 4th row starting at row 3) — very cheap.
#[inline]
fn apply_scanline_glow(buffer: &mut [u32], width: usize, height: usize, glow_strength: u32) {
    if glow_strength == 0 || height < 8 { return; }
    
    // glow_strength 0-100 → blend factor 0-64
    let blend = (glow_strength * 64 / 100).min(64) as u32;
    if blend == 0 { return; }
    
    // Only process scanline gap rows (row % 4 == 3) and slight dim rows (row % 4 == 2)
    for y in 0..height {
        if y % 4 != 3 && y % 4 != 2 { continue; }
        
        let factor = if y % 4 == 3 { blend } else { blend >> 1 }; // gap gets full glow, dim row gets half
        let inv = 256 - factor;
        
        // Average the rows above and below
        let above = if y > 0 { y - 1 } else { y };
        let below = if y + 1 < height { y + 1 } else { y };
        
        let row = y * width;
        let above_row = above * width;
        let below_row = below * width;
        
        for x in 0..width {
            let idx = row + x;
            let pixel = unsafe { *buffer.get_unchecked(idx) };
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;
            
            let pa = unsafe { *buffer.get_unchecked(above_row + x) };
            let pb = unsafe { *buffer.get_unchecked(below_row + x) };
            
            // Average of above and below
            let avg_r = (((pa >> 16) & 0xFF) + ((pb >> 16) & 0xFF)) >> 1;
            let avg_g = (((pa >> 8) & 0xFF) + ((pb >> 8) & 0xFF)) >> 1;
            let avg_b = ((pa & 0xFF) + (pb & 0xFF)) >> 1;
            
            // Blend: pixel * inv + avg * factor, all >>8
            let nr = ((r * inv + avg_r * factor) >> 8).min(255);
            let ng = ((g * inv + avg_g * factor) >> 8).min(255);
            let nb = ((b * inv + avg_b * factor) >> 8).min(255);
            
            unsafe { *buffer.get_unchecked_mut(idx) = (nr << 16) | (ng << 8) | nb; }
        }
    }
}

fn crt_filter(input: &[u32], output: &mut Vec<u32>, vignette_table: &[u16], distortion_table: &[(u32, u32)], crt_cfg: &CrtConfig, mask_table: &[(u8, u8, u8)]) {
    output.resize(SCREEN_W * SCREEN_H, 0);

    // Pre-compute all coefficients once
    let si = crt_cfg.scanline_intensity as u32;
    let scan_muls: [u32; 4] = [
        255,                                    // row 0: full bright
        255 - (si * 15 / 100),                  // row 1: slight dim
        255 - (si * 25 / 100),                  // row 2: moderate dim
        255 - si.min(255) * 55 / 100,           // row 3: scanline gap (stronger)
    ];

    let pw = crt_cfg.phosphor_warmth as u32;
    let pr_mul = 256 + (pw * 24 / 100);
    let pg_mul = 256 - (pw * 8 / 100);
    let pb_mul = 256 - (pw * 36 / 100);

    let blur_side = (25u32 * crt_cfg.blur_amount as u32) / 40;
    let blur_center = 256 - blur_side * 2;
    let use_blur = blur_side > 0;

    let use_mask = crt_cfg.mask_mode != CrtMaskMode::Off;
    let mask_intensity = crt_cfg.mask_intensity as u32;

    // Hoist branching outside the pixel loop - create specialized paths
    if use_mask && use_blur {
        // Full pipeline: blur + mask
        crt_filter_full(input, output, vignette_table, distortion_table, 
                       &scan_muls, pr_mul, pg_mul, pb_mul, 
                       blur_center, blur_side, mask_table, mask_intensity);
    } else if use_mask {
        // Mask but no blur
        crt_filter_masked(input, output, vignette_table, distortion_table, 
                         &scan_muls, pr_mul, pg_mul, pb_mul, mask_table, mask_intensity);
    } else if use_blur {
        // Blur but no mask
        crt_filter_blurred(input, output, vignette_table, distortion_table, 
                          &scan_muls, pr_mul, pg_mul, pb_mul, 
                          blur_center, blur_side);
    } else {
        // Basic pipeline: no blur, no mask
        crt_filter_basic(input, output, vignette_table, distortion_table, 
                        &scan_muls, pr_mul, pg_mul, pb_mul);
    }
}

// Specialized path: full pipeline with blur and mask
#[inline(always)]
fn crt_filter_full(input: &[u32], output: &mut Vec<u32>, vignette_table: &[u16], 
                   distortion_table: &[(u32, u32)], scan_muls: &[u32; 4],
                   pr_mul: u32, pg_mul: u32, pb_mul: u32,
                   blur_center: u32, blur_side: u32, mask_table: &[(u8, u8, u8)], mask_intensity: u32) {
    for dst_y in 0..SCREEN_H {
        let dst_row = dst_y * SCREEN_W;
        let scan_mul = scan_muls[dst_y % 4];
        
        for dst_x in 0..SCREEN_W {
            let table_idx = dst_row + dst_x;
            let (src_xf, src_yf) = unsafe { *distortion_table.get_unchecked(table_idx) };
            
            if src_xf == 0xFFFFFFFF {
                unsafe { *output.get_unchecked_mut(table_idx) = 0; }
                continue;
            }
            
            let src_x0 = (src_xf >> 8) as usize;
            let src_y0 = (src_yf >> 8) as usize;
            let src_x1 = (src_x0 + 1).min(255);
            let src_y1 = (src_y0 + 1).min(239);
            let frac_x = if src_x0 >= 255 { 0 } else { (src_xf & 0xFF) as u32 };
            let frac_y = if src_y0 >= 239 { 0 } else { (src_yf & 0xFF) as u32 };
            
            let base_offset = src_y0 * 256;
            let p00 = unsafe { *input.get_unchecked(base_offset + src_x0) };
            let p10 = unsafe { *input.get_unchecked(base_offset + src_x1) };
            let p01 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x0) };
            let p11 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x1) };
            
            let (mut r, mut g, mut b) = blend_bilinear_rgb(p00, p10, p01, p11, frac_x, frac_y);
            
            // Apply CRT gamma — crushes darks, enriches midtones
            r = GAMMA_TABLE[r.min(255) as usize] as u32;
            g = GAMMA_TABLE[g.min(255) as usize] as u32;
            b = GAMMA_TABLE[b.min(255) as usize] as u32;
            
            // Blur
            if src_x0 > 0 && src_x0 < 255 {
                let left = unsafe { *input.get_unchecked(base_offset + src_x0 - 1) };
                let right = unsafe { *input.get_unchecked(base_offset + src_x1) };
                (r, g, b) = apply_blur_3tap(r, g, b, left, right, blur_center, blur_side);
            }
            
            (r, g, b) = apply_phosphor(r, g, b, pr_mul, pg_mul, pb_mul);
            let vig = unsafe { *vignette_table.get_unchecked(table_idx) as u32 };
            (r, g, b) = apply_scanline_vignette(r, g, b, scan_mul, vig);
            
            let (mr, mg, mb) = unsafe { *mask_table.get_unchecked(table_idx) };
            (r, g, b) = apply_mask(r, g, b, mr, mg, mb, mask_intensity);
            
            unsafe { *output.get_unchecked_mut(table_idx) = pack_rgb(r, g, b); }
        }
    }
}

// Specialized path: mask only, no blur
#[inline(always)]
fn crt_filter_masked(input: &[u32], output: &mut Vec<u32>, vignette_table: &[u16], 
                     distortion_table: &[(u32, u32)], scan_muls: &[u32; 4],
                     pr_mul: u32, pg_mul: u32, pb_mul: u32, mask_table: &[(u8, u8, u8)], mask_intensity: u32) {
    for dst_y in 0..SCREEN_H {
        let dst_row = dst_y * SCREEN_W;
        let scan_mul = scan_muls[dst_y % 4];
        
        for dst_x in 0..SCREEN_W {
            let table_idx = dst_row + dst_x;
            let (src_xf, src_yf) = unsafe { *distortion_table.get_unchecked(table_idx) };
            
            if src_xf == 0xFFFFFFFF {
                unsafe { *output.get_unchecked_mut(table_idx) = 0; }
                continue;
            }
            
            let src_x0 = (src_xf >> 8) as usize;
            let src_y0 = (src_yf >> 8) as usize;
            let src_x1 = (src_x0 + 1).min(255);
            let src_y1 = (src_y0 + 1).min(239);
            let frac_x = if src_x0 >= 255 { 0 } else { (src_xf & 0xFF) as u32 };
            let frac_y = if src_y0 >= 239 { 0 } else { (src_yf & 0xFF) as u32 };
            
            let base_offset = src_y0 * 256;
            let p00 = unsafe { *input.get_unchecked(base_offset + src_x0) };
            let p10 = unsafe { *input.get_unchecked(base_offset + src_x1) };
            let p01 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x0) };
            let p11 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x1) };
            
            let (mut r, mut g, mut b) = blend_bilinear_rgb(p00, p10, p01, p11, frac_x, frac_y);
            
            // Apply CRT gamma — crushes darks, enriches midtones
            r = GAMMA_TABLE[r.min(255) as usize] as u32;
            g = GAMMA_TABLE[g.min(255) as usize] as u32;
            b = GAMMA_TABLE[b.min(255) as usize] as u32;
            
            (r, g, b) = apply_phosphor(r, g, b, pr_mul, pg_mul, pb_mul);
            let vig = unsafe { *vignette_table.get_unchecked(table_idx) as u32 };
            (r, g, b) = apply_scanline_vignette(r, g, b, scan_mul, vig);
            
            let (mr, mg, mb) = unsafe { *mask_table.get_unchecked(table_idx) };
            (r, g, b) = apply_mask(r, g, b, mr, mg, mb, mask_intensity);
            
            unsafe { *output.get_unchecked_mut(table_idx) = pack_rgb(r, g, b); }
        }
    }
}

// Specialized path: blur only, no mask
#[inline(always)]
fn crt_filter_blurred(input: &[u32], output: &mut Vec<u32>, vignette_table: &[u16], 
                      distortion_table: &[(u32, u32)], scan_muls: &[u32; 4],
                      pr_mul: u32, pg_mul: u32, pb_mul: u32,
                      blur_center: u32, blur_side: u32) {
    for dst_y in 0..SCREEN_H {
        let dst_row = dst_y * SCREEN_W;
        let scan_mul = scan_muls[dst_y % 4];
        
        for dst_x in 0..SCREEN_W {
            let table_idx = dst_row + dst_x;
            let (src_xf, src_yf) = unsafe { *distortion_table.get_unchecked(table_idx) };
            
            if src_xf == 0xFFFFFFFF {
                unsafe { *output.get_unchecked_mut(table_idx) = 0; }
                continue;
            }
            
            let src_x0 = (src_xf >> 8) as usize;
            let src_y0 = (src_yf >> 8) as usize;
            let src_x1 = (src_x0 + 1).min(255);
            let src_y1 = (src_y0 + 1).min(239);
            let frac_x = if src_x0 >= 255 { 0 } else { (src_xf & 0xFF) as u32 };
            let frac_y = if src_y0 >= 239 { 0 } else { (src_yf & 0xFF) as u32 };
            
            let base_offset = src_y0 * 256;
            let p00 = unsafe { *input.get_unchecked(base_offset + src_x0) };
            let p10 = unsafe { *input.get_unchecked(base_offset + src_x1) };
            let p01 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x0) };
            let p11 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x1) };
            
            let (mut r, mut g, mut b) = blend_bilinear_rgb(p00, p10, p01, p11, frac_x, frac_y);
            
            // Apply CRT gamma — crushes darks, enriches midtones
            r = GAMMA_TABLE[r.min(255) as usize] as u32;
            g = GAMMA_TABLE[g.min(255) as usize] as u32;
            b = GAMMA_TABLE[b.min(255) as usize] as u32;
            
            // Blur
            if src_x0 > 0 && src_x0 < 255 {
                let left = unsafe { *input.get_unchecked(base_offset + src_x0 - 1) };
                let right = unsafe { *input.get_unchecked(base_offset + src_x1) };
                (r, g, b) = apply_blur_3tap(r, g, b, left, right, blur_center, blur_side);
            }
            
            (r, g, b) = apply_phosphor(r, g, b, pr_mul, pg_mul, pb_mul);
            let vig = unsafe { *vignette_table.get_unchecked(table_idx) as u32 };
            (r, g, b) = apply_scanline_vignette(r, g, b, scan_mul, vig);
            
            unsafe { *output.get_unchecked_mut(table_idx) = pack_rgb(r, g, b); }
        }
    }
}

// Specialized path: basic (no blur, no mask)
#[inline(always)]
fn crt_filter_basic(input: &[u32], output: &mut Vec<u32>, vignette_table: &[u16], 
                    distortion_table: &[(u32, u32)], scan_muls: &[u32; 4],
                    pr_mul: u32, pg_mul: u32, pb_mul: u32) {
    for dst_y in 0..SCREEN_H {
        let dst_row = dst_y * SCREEN_W;
        let scan_mul = scan_muls[dst_y % 4];
        
        for dst_x in 0..SCREEN_W {
            let table_idx = dst_row + dst_x;
            let (src_xf, src_yf) = unsafe { *distortion_table.get_unchecked(table_idx) };
            
            if src_xf == 0xFFFFFFFF {
                unsafe { *output.get_unchecked_mut(table_idx) = 0; }
                continue;
            }
            
            let src_x0 = (src_xf >> 8) as usize;
            let src_y0 = (src_yf >> 8) as usize;
            let src_x1 = (src_x0 + 1).min(255);
            let src_y1 = (src_y0 + 1).min(239);
            let frac_x = if src_x0 >= 255 { 0 } else { (src_xf & 0xFF) as u32 };
            let frac_y = if src_y0 >= 239 { 0 } else { (src_yf & 0xFF) as u32 };
            
            let base_offset = src_y0 * 256;
            let p00 = unsafe { *input.get_unchecked(base_offset + src_x0) };
            let p10 = unsafe { *input.get_unchecked(base_offset + src_x1) };
            let p01 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x0) };
            let p11 = unsafe { *input.get_unchecked(src_y1 * 256 + src_x1) };
            
            let (mut r, mut g, mut b) = blend_bilinear_rgb(p00, p10, p01, p11, frac_x, frac_y);
            
            // Apply CRT gamma — crushes darks, enriches midtones
            r = GAMMA_TABLE[r.min(255) as usize] as u32;
            g = GAMMA_TABLE[g.min(255) as usize] as u32;
            b = GAMMA_TABLE[b.min(255) as usize] as u32;
            
            (r, g, b) = apply_phosphor(r, g, b, pr_mul, pg_mul, pb_mul);
            let vig = unsafe { *vignette_table.get_unchecked(table_idx) as u32 };
            (r, g, b) = apply_scanline_vignette(r, g, b, scan_mul, vig);
            
            unsafe { *output.get_unchecked_mut(table_idx) = pack_rgb(r, g, b); }
        }
    }
}

fn scale_simple(input: &[u32], output: &mut Vec<u32>) {
    
    output.resize(SCREEN_W * SCREEN_H, 0);
    for y in 0..SCREEN_H {
        let src_y = y * 240 / SCREEN_H;
        for x in 0..SCREEN_W {
            let src_x = x * 256 / SCREEN_W;
            output[y * SCREEN_W + x] = input[src_y * 256 + src_x.min(255)];
        }
    }
}

fn build_tv_frame(frame: &mut Vec<u32>) {
    frame.resize(WINDOW_WIDTH * WINDOW_HEIGHT, 0);
    
    // ===== WALL BACKGROUND =====
    for y in 0..TV_HEIGHT {
        for x in 0..WINDOW_WIDTH {
            let idx = y * WINDOW_WIDTH + x;
            let noise = ((x.wrapping_mul(7) ^ y.wrapping_mul(13)) % 4) as u32;
            frame[idx] = (0x1A + noise) << 16 | (0x18 + noise) << 8 | (0x16 + noise);
        }
    }
    
    // ===== TV OUTER SHELL (rounded rectangle) =====
    let tv_x1: usize = 50;
    let tv_y1: usize = 15;
    let tv_x2: usize = WINDOW_WIDTH - 50;
    let tv_y2: usize = TV_HEIGHT - 15;
    let tv_w = tv_x2 - tv_x1;
    let tv_h = tv_y2 - tv_y1;
    let corner_r: usize = 20;
    
    for y in tv_y1..tv_y2 {
        for x in tv_x1..tv_x2 {
            let lx = x - tv_x1;
            let ly = y - tv_y1;
            
            // Rounded corner check
            let in_corner = (lx < corner_r && ly < corner_r) ||
                           (lx >= tv_w - corner_r && ly < corner_r) ||
                           (lx < corner_r && ly >= tv_h - corner_r) ||
                           (lx >= tv_w - corner_r && ly >= tv_h - corner_r);
            
            if in_corner {
                let cx = if lx < corner_r { corner_r } else { tv_w - corner_r };
                let cy = if ly < corner_r { corner_r } else { tv_h - corner_r };
                let dx = lx as f32 - cx as f32;
                let dy = ly as f32 - cy as f32;
                if (dx * dx + dy * dy).sqrt() > corner_r as f32 {
                    continue; // Outside rounded corner
                }
            }
            
            // Vertical gradient for 3D depth (lighter at top)
            let vert_t = ly as f32 / tv_h as f32;
            let base_r = (0x38 as f32 * (1.0 - vert_t * 0.4)) as u32;
            let base_g = (0x38 as f32 * (1.0 - vert_t * 0.4)) as u32;
            let base_b = (0x3C as f32 * (1.0 - vert_t * 0.4)) as u32;
            
            // Subtle plastic texture noise
            let noise = ((x.wrapping_mul(31) ^ y.wrapping_mul(17)) % 3) as u32;
            let r = base_r.saturating_add(noise).min(255);
            let g = base_g.saturating_add(noise).min(255);
            let b = base_b.saturating_add(noise).min(255);
            
            frame[y * WINDOW_WIDTH + x] = (r << 16) | (g << 8) | b;
        }
    }
    
    // ===== EDGE HIGHLIGHTS (3D depth on shell) =====
    // Top edge highlight (light catch from above)
    for x in (tv_x1 + corner_r)..(tv_x2 - corner_r) {
        for dy in 0..3 {
            let y = tv_y1 + dy;
            let brightness = (50 - dy * 15) as u32;
            let idx = y * WINDOW_WIDTH + x;
            let p = frame[idx];
            let r = (((p >> 16) & 0xFF) + brightness).min(255);
            let g = (((p >> 8) & 0xFF) + brightness).min(255);
            let b = ((p & 0xFF) + brightness).min(255);
            frame[idx] = (r << 16) | (g << 8) | b;
        }
    }
    // Left edge highlight
    for y in (tv_y1 + corner_r)..(tv_y2 - corner_r) {
        for dx in 0..2 {
            let x = tv_x1 + dx;
            let brightness = (30 - dx * 15) as u32;
            let idx = y * WINDOW_WIDTH + x;
            let p = frame[idx];
            let r = (((p >> 16) & 0xFF) + brightness).min(255);
            let g = (((p >> 8) & 0xFF) + brightness).min(255);
            let b = ((p & 0xFF) + brightness).min(255);
            frame[idx] = (r << 16) | (g << 8) | b;
        }
    }
    // Bottom/right edges darker (shadow)
    for x in (tv_x1 + corner_r)..(tv_x2 - corner_r) {
        for dy in 0..2 {
            let y = tv_y2 - 1 - dy;
            let idx = y * WINDOW_WIDTH + x;
            let p = frame[idx];
            let r = ((p >> 16) & 0xFF).saturating_sub(20);
            let g = ((p >> 8) & 0xFF).saturating_sub(20);
            let b = (p & 0xFF).saturating_sub(20);
            frame[idx] = (r << 16) | (g << 8) | b;
        }
    }
    
    // ===== SCREEN BEZEL (inset shadow around screen) =====
    let bezel_pad: usize = 12;
    let bx1 = SCREEN_X - bezel_pad;
    let by1 = SCREEN_Y - bezel_pad;
    let bx2 = SCREEN_X + SCREEN_W + bezel_pad;
    let by2 = SCREEN_Y + SCREEN_H + bezel_pad;
    
    for y in by1..by2 {
        for x in bx1..bx2 {
            if y >= SCREEN_Y && y < SCREEN_Y + SCREEN_H && x >= SCREEN_X && x < SCREEN_X + SCREEN_W {
                continue; // Don't touch screen area
            }
            if y < TV_HEIGHT && x < WINDOW_WIDTH {
                // Inset effect: top/left darker, bottom/right lighter
                let dist_top = (y - by1) as f32;
                let dist_left = (x - bx1) as f32;
                let dist_bottom = (by2 - 1 - y) as f32;
                let dist_right = (bx2 - 1 - x) as f32;
                
                let min_dist = dist_top.min(dist_left).min(dist_bottom).min(dist_right);
                let depth = (min_dist / bezel_pad as f32).min(1.0);
                
                // Dark inset
                let base = 0x18 as f32;
                let shade = base + (depth * 10.0);
                let r = shade as u32;
                let g = shade as u32;
                let b = (shade + 2.0) as u32;
                frame[y * WINDOW_WIDTH + x] = (r << 16) | (g << 8) | b;
            }
        }
    }
    
    // ===== GLASS EDGE (crisp 2px border around screen) =====
    let glass_color: u32 = 0x080808;
    for x in (SCREEN_X - 2)..(SCREEN_X + SCREEN_W + 2) {
        for dy in 0..2 {
            // Top glass edge
            let y = SCREEN_Y - 2 + dy;
            if y < TV_HEIGHT { frame[y * WINDOW_WIDTH + x] = glass_color; }
            // Bottom glass edge
            let y = SCREEN_Y + SCREEN_H + dy;
            if y < TV_HEIGHT { frame[y * WINDOW_WIDTH + x] = glass_color; }
        }
    }
    for y in (SCREEN_Y - 2)..(SCREEN_Y + SCREEN_H + 2) {
        for dx in 0..2 {
            // Left glass edge
            let x = SCREEN_X - 2 + dx;
            if x < WINDOW_WIDTH && y < TV_HEIGHT { frame[y * WINDOW_WIDTH + x] = glass_color; }
            // Right glass edge
            let x = SCREEN_X + SCREEN_W + dx;
            if x < WINDOW_WIDTH && y < TV_HEIGHT { frame[y * WINDOW_WIDTH + x] = glass_color; }
        }
    }
    
    // ===== ACCENT LINE (subtle brand area below screen) =====
    let accent_y = SCREEN_Y + SCREEN_H + bezel_pad + 20;
    let accent_w: usize = 200;
    let accent_x = SCREEN_X + (SCREEN_W - accent_w) / 2;
    if accent_y < TV_HEIGHT {
        for x in accent_x..(accent_x + accent_w) {
            frame[accent_y * WINDOW_WIDTH + x] = 0x4A4A4E;
        }
    }
    
    // ===== POWER INDICATOR (tiny green dot, bottom-left) =====
    let led_x = tv_x1 + 60;
    let led_y = tv_y2 - 30;
    if led_y < TV_HEIGHT {
        for dy in 0..3usize {
            for dx in 0..3usize {
                let x = led_x + dx;
                let y = led_y + dy;
                if y < TV_HEIGHT && x < WINDOW_WIDTH {
                    // Bright green center, dimmer edges
                    let dist = ((dx as f32 - 1.0).powi(2) + (dy as f32 - 1.0).powi(2)).sqrt();
                    if dist < 1.5 {
                        let g = (0x80 as f32 * (1.0 - dist / 2.0)) as u32;
                        frame[y * WINDOW_WIDTH + x] = (g.min(255)) << 8;
                    }
                }
            }
        }
    }
    
    // ===== DROP SHADOW (below TV onto wall) =====
    for dy in 0..8usize {
        let y = tv_y2 + dy;
        if y >= TV_HEIGHT { break; }
        let alpha = (8 - dy) as f32 / 12.0;
        for x in (tv_x1 + 10)..(tv_x2 - 10) {
            let idx = y * WINDOW_WIDTH + x;
            let p = frame[idx];
            let r = ((((p >> 16) & 0xFF) as f32) * (1.0 - alpha)) as u32;
            let g = ((((p >> 8) & 0xFF) as f32) * (1.0 - alpha)) as u32;
            let b = (((p & 0xFF) as f32) * (1.0 - alpha)) as u32;
            frame[idx] = (r << 16) | (g << 8) | b;
        }
    }
}

#[inline(always)]
fn sq_dist(x1: usize, y1: usize, x2: usize, y2: usize) -> usize {
    let dx = if x1 > x2 { x1 - x2 } else { x2 - x1 };
    let dy = if y1 > y2 { y1 - y2 } else { y2 - y1 };
    dx * dx + dy * dy
}

fn composite_screen_fast(result: &mut [u32], game_output: &[u32], window_width: usize) {
    // Only blit the CRT screen area onto the persistent composite buffer.
    // The TV frame was already baked in at init time — no full-frame copy needed.
    for src_y in 0..SCREEN_H {
        let dst_row_start = (src_y + SCREEN_Y) * window_width + SCREEN_X;
        let src_row_start = src_y * SCREEN_W;
        let dst_slice = &mut result[dst_row_start..dst_row_start + SCREEN_W];
        let src_slice = &game_output[src_row_start..src_row_start + SCREEN_W];
        dst_slice.copy_from_slice(src_slice);
    }
}

fn build_glare_table() -> Vec<u8> {
    let mut table = vec![0u8; SCREEN_W * SCREEN_H];

    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            let fx = x as f64 / SCREEN_W as f64;  // 0..1
            let fy = y as f64 / SCREEN_H as f64;  // 0..1
            let nx = fx * 2.0 - 1.0;  // -1..1
            let ny = fy * 2.0 - 1.0;  // -1..1

            // Edge distance for Fresnel and fading
            let edge_dist = nx.abs().max(ny.abs()).min(1.0);

            // Layer 1: Enhanced Fresnel edge reflection (Schlick approximation)
            // Starts earlier (0.5) for more gradual glass-edge glow
            let fresnel_t = ((edge_dist - 0.5).max(0.0) / 0.5).min(1.0);
            let fresnel = fresnel_t.powi(3) * 22.0;

            // Layer 2: Primary specular — overhead light reflecting off curved glass
            // Elongated horizontally (like a fluorescent tube reflection)
            let spec1_x = (fx - 0.35) / 0.18;
            let spec1_y = (fy - 0.18) / 0.08;  // Narrow vertically = elongated highlight
            let spec1 = (-(spec1_x * spec1_x + spec1_y * spec1_y) / 2.0).exp() * 45.0;

            // Layer 3: Secondary specular — smaller, dimmer (second light or bounce)
            let spec2_x = (fx - 0.68) / 0.12;
            let spec2_y = (fy - 0.15) / 0.06;
            let spec2 = (-(spec2_x * spec2_x + spec2_y * spec2_y) / 2.0).exp() * 25.0;

            // Layer 4: Broad curved reflection arc (the characteristic CRT glass sweep)
            // This is the wide, gentle arc you see on real CRT screens
            let arc_center = 0.3 - 0.15 * nx * nx;  // Curved arc following glass curvature
            let arc_dist = (fy - arc_center).abs();
            let arc = (-(arc_dist * arc_dist) * 35.0).exp() * 15.0;
            let arc_fade = (1.0 - (nx.abs() - 0.8).max(0.0) * 5.0).max(0.0);  // Fade at sides
            let arc = arc * arc_fade;

            // Layer 5: Subtle bottom-edge reflection (desk/surface bounce light)
            let bottom_glow_t = ((fy - 0.85).max(0.0) / 0.15).min(1.0);
            let bottom_center = (-(nx * nx) * 3.0).exp();
            let bottom = bottom_glow_t * bottom_center * 10.0;

            // Layer 6: Very faint window reflection (rectangular ghost, upper area)
            let win_x = ((fx - 0.45).abs() < 0.15) as u8 as f64;
            let win_y = ((fy > 0.08) && (fy < 0.35)) as u8 as f64;
            let win_edge_x = (1.0 - ((fx - 0.45).abs() / 0.15).powi(4)).max(0.0);
            let win_edge_y_top = ((fy - 0.08).min(0.05) / 0.05).max(0.0);
            let win_edge_y_bot = ((0.35 - fy).min(0.05) / 0.05).max(0.0);
            let window = win_x * win_y * win_edge_x * win_edge_y_top * win_edge_y_bot * 8.0;

            // Combine all layers
            let total = (fresnel + spec1 + spec2 + arc + bottom + window).max(0.0).min(70.0) as u8;

            // Zero out near border (glass-bezel junction has no glare)
            let in_border = x < 4 || x >= SCREEN_W - 4 || y < 4 || y >= SCREEN_H - 4;
            table[y * SCREEN_W + x] = if in_border { 0 } else { total };
        }
    }
    table
}

// Enhanced glass overlay: tint + specular glare for realistic CRT glass
fn build_glass_thickness_table() -> Vec<u16> {
    let mut table = vec![0u16; SCREEN_W * SCREEN_H];
    for y in 0..SCREEN_H {
        let fy = (y as f64 / SCREEN_H as f64) * 2.0 - 1.0;
        let edge_y = ((fy.abs() - 0.3).max(0.0) / 0.7 * 256.0) as u16;
        for x in 0..SCREEN_W {
            let fx = (x as f64 / SCREEN_W as f64) * 2.0 - 1.0;
            let edge_x = ((fx.abs() - 0.3).max(0.0) / 0.7 * 256.0) as u16;
            table[y * SCREEN_W + x] = (edge_x + edge_y) / 2;
        }
    }
    table
}

fn build_ghost_alpha_table(glass_intensity: u8) -> Vec<u8> {
    let mut table = vec![0u8; SCREEN_W * SCREEN_H];
    if glass_intensity < 20 { return table; }
    let base_alpha = ((glass_intensity as f64) - 20.0) * 16.0 / 80.0;
    if base_alpha <= 0.0 { return table; }

    for y in 0..SCREEN_H {
        let fy = ((y as f64 / SCREEN_H as f64) * 2.0 - 1.0).abs();
        let edge_boost_y = (1.0 + (fy - 0.4).max(0.0) * 2.0).min(2.5);
        for x in 0..SCREEN_W {
            let fx = ((x as f64 / SCREEN_W as f64) * 2.0 - 1.0).abs();
            let edge_boost_x = (1.0 + (fx - 0.4).max(0.0) * 2.0).min(2.5);
            let local_alpha = base_alpha * (edge_boost_x + edge_boost_y) / 2.0;
            table[y * SCREEN_W + x] = (local_alpha as u8).min(40);
        }
    }
    table
}

fn apply_screen_glare(buffer: &mut [u32], glare_table: &[u8], thickness_table: &[u16], window_width: usize, glass_intensity: u8) {
    if glass_intensity == 0 { return; }

    const CORNER_R: usize = 12;
    const CORNER_R_SQ: usize = CORNER_R * CORNER_R;
    let intensity_factor = glass_intensity as u32;

    // Glass tint: CRT glass slightly reduces contrast and adds warmth
    // At intensity 100: ~4% contrast reduction + slight warm shift
    let tint_strength = intensity_factor * 10 / 100;  // 0-10 range

    let corner_x_max = SCREEN_W - CORNER_R;
    let corner_y_max = SCREEN_H - CORNER_R;

    for y in 0..SCREEN_H {
        let buf_row = (y + SCREEN_Y) * window_width + SCREEN_X;
        let glare_row = y * SCREEN_W;

        let in_corner_y_top = y < CORNER_R;
        let in_corner_y_bottom = y >= corner_y_max;

        for x in 0..SCREEN_W {
            // Corner rounding check (keep existing logic)
            if (in_corner_y_top || in_corner_y_bottom) && (x < CORNER_R || x >= corner_x_max) {
                let (cx, cy) = if x < CORNER_R {
                    if in_corner_y_top { (CORNER_R, CORNER_R) } else { (CORNER_R, SCREEN_H - 1 - CORNER_R) }
                } else {
                    if in_corner_y_top { (SCREEN_W - 1 - CORNER_R, CORNER_R) } else { (SCREEN_W - 1 - CORNER_R, SCREEN_H - 1 - CORNER_R) }
                };
                if sq_dist(x, y, cx, cy) > CORNER_R_SQ { continue; }
            }

            let idx = buf_row + x;
            let pixel = unsafe { *buffer.get_unchecked(idx) };
            let mut r = (pixel >> 16) & 0xFF;
            let mut g = (pixel >> 8) & 0xFF;
            let mut b = pixel & 0xFF;

            // Glass tint: slight contrast reduction + warm color shift from glass
            // Real CRT glass absorbs some light, especially at edges (thicker glass)
            if tint_strength > 0 {
                let thickness = 256 + unsafe { *thickness_table.get_unchecked(glare_row + x) } as u32;
                let tint = tint_strength * thickness / 256;

                // Pull toward a neutral grey (reduces contrast) with slight warm bias
                let grey = ((r + g + b) * 171) >> 9;          // /3 via multiply-shift
                r = r + ((grey.saturating_sub(r) * tint * 205) >> 13);   // /40 via multiply-shift
                g = g + ((grey.saturating_sub(g) * tint * 182) >> 13);   // /45 via multiply-shift
                b = b.saturating_sub((tint * 171) >> 9);       // /3 via multiply-shift
            }

            // Specular glare overlay
            let glare_base = unsafe { *glare_table.get_unchecked(glare_row + x) as u32 };
            if glare_base > 0 {
                let brightness = ((r + g + b) * 171) >> 9;    // /3 via multiply-shift
                let glare = (glare_base * intensity_factor * (200_u32.saturating_sub(brightness)) * 29) >> 19;  // /18000 via multiply-shift

                // Glare is slightly warm (real reflections pick up room light color)
                r = (r + glare + ((glare * 171) >> 11)).min(255);   // glare/12 via multiply-shift
                g = (g + glare).min(255);
                b = (b + glare.saturating_sub((glare * 17) >> 8)).min(255);  // glare/15 via multiply-shift
            }

            unsafe { *buffer.get_unchecked_mut(idx) = (r << 16) | (g << 8) | b; }
        }
    }
}

/// Internal reflection: thick CRT glass creates a faint ghost image
/// shifted slightly diagonally. More visible near screen edges.
#[inline]
fn apply_internal_ghost(buffer: &mut [u32], source_copy: &[u32], ghost_alpha_table: &[u8], window_width: usize) {
    let shift_x: usize = 3;
    let shift_y: usize = 2;

    for y in 0..SCREEN_H.saturating_sub(shift_y) {
        let buf_row = (y + SCREEN_Y) * window_width + SCREEN_X;
        let src_row = (y + shift_y + SCREEN_Y) * window_width + SCREEN_X;
        let alpha_row = y * SCREEN_W;

        for x in 0..SCREEN_W.saturating_sub(shift_x) {
            let local_alpha = unsafe { *ghost_alpha_table.get_unchecked(alpha_row + x) } as u32;
            if local_alpha == 0 { continue; }

            let idx = buf_row + x;
            let src_idx = src_row + x + shift_x;

            let pixel = unsafe { *buffer.get_unchecked(idx) };
            let ghost_pixel = unsafe { *source_copy.get_unchecked(src_idx) };

            let inv_alpha = 256 - local_alpha;
            let r = (((pixel >> 16) & 0xFF) * inv_alpha + ((ghost_pixel >> 16) & 0xFF) * local_alpha) >> 8;
            let g = (((pixel >> 8) & 0xFF) * inv_alpha + ((ghost_pixel >> 8) & 0xFF) * local_alpha) >> 8;
            let b = ((pixel & 0xFF) * inv_alpha + (ghost_pixel & 0xFF) * local_alpha) >> 8;

            unsafe { *buffer.get_unchecked_mut(idx) = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255); }
        }
    }
}

/// Per-pixel CA shift offsets, precomputed once at startup (or when glass_intensity changes).
/// Each entry: (shift_x, shift_y) as i16 in sub-pixel units.
/// Positive shift means red channel shifts outward, blue inward (or vice-versa).
struct CaTable {
    /// For each pixel index (y * width + x): (shift_x, shift_y) in pixels (i16).
    shifts: Vec<(i16, i16)>,
}

fn build_ca_table(width: usize, height: usize, glass_intensity: u8) -> CaTable {
    let intensity_factor = glass_intensity as f64 / 100.0;
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let max_shift = 1.5; // max pixel shift at full intensity at edges
    let mut shifts = vec![(0i16, 0i16); width * height];

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f64 - cx) / cx; // normalized -1..1
            let dy = (y as f64 - cy) / cy;
            let edge_sq = (dx * dx + dy * dy).min(1.0);
            let edge_factor = edge_sq.sqrt();

            if edge_factor > 0.85 {
                let strength = ((edge_factor - 0.85) / 0.15).min(1.0) * intensity_factor;
                let sx = (dx * edge_factor * max_shift * strength) as i16;
                let sy = (dy * edge_factor * max_shift * strength) as i16;
                if sx != 0 || sy != 0 {
                    shifts[y * width + x] = (sx, sy);
                }
            }
        }
    }

    CaTable { shifts }
}

// Optimized: Use unsafe for bounds-checked accesses, inline function
#[inline]
fn apply_chromatic_aberration(buffer: &mut [u32], source: &[u32], ca_table: &CaTable, width: usize, height: usize) {
    let w = width as i32;
    let h = height as i32;

    for y in 0..height {
        let row = y * width;
        for x in 0..width {
            let (sx, sy) = unsafe { *ca_table.shifts.get_unchecked(row + x) };
            if sx == 0 && sy == 0 { continue; }

            let r_x = ((x as i32) - sx as i32).clamp(0, w - 1) as usize;
            let r_y = ((y as i32) - sy as i32).clamp(0, h - 1) as usize;
            let b_x = ((x as i32) + sx as i32).clamp(0, w - 1) as usize;
            let b_y = ((y as i32) + sy as i32).clamp(0, h - 1) as usize;

            let r = unsafe { (*source.get_unchecked(r_y * width + r_x) >> 16) & 0xFF };
            let g = unsafe { (*source.get_unchecked(row + x) >> 8) & 0xFF };
            let b = unsafe { *source.get_unchecked(b_y * width + b_x) & 0xFF };

            unsafe { *buffer.get_unchecked_mut(row + x) = (r << 16) | (g << 8) | b; }
        }
    }
}

fn get_small_glyph(ch: char) -> [u8; 5] {
    match ch {
        'A' => [0b111, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b111],
        'K' => [0b101, 0b110, 0b100, 0b110, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'Q' => [0b111, 0b101, 0b101, 0b111, 0b001],
        'R' => [0b111, 0b101, 0b111, 0b110, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        _ => [0b000, 0b000, 0b000, 0b000, 0b000],
    }
}

fn draw_text(frame: &mut Vec<u32>, text: &str, start_x: usize, start_y: usize, color: u32, stride: usize) {
    let mut cursor_x = start_x;
    for ch in text.chars() {
        let glyph = get_small_glyph(ch);
        for (row, &bits) in glyph.iter().enumerate() {
            for col in 0..3 {
                if bits & (0b100 >> col) != 0 {
                    let px = cursor_x + col;
                    let py = start_y + row;
                    if px < stride && py * stride + px < frame.len() {
                        frame[py * stride + px] = color;
                    }
                }
            }
        }
        cursor_x += 4; // 3px char + 1px gap
    }
}

fn build_console_overlay(frame: &mut Vec<u32>, tv_h: usize, win_w: usize, win_h: usize) {
    // Simple dark shelf/stand below TV
    for y in tv_h..win_h {
        for x in 0..win_w {
            let idx = y * win_w + x;
            if idx < frame.len() {
                let dy = y - tv_h;
                // Gradient: darker at top (shadow from TV), lighter below
                let shadow = if dy < 20 {
                    (20 - dy) as f32 / 30.0
                } else {
                    0.0
                };
                
                let base_r = 0x22 as f32 * (1.0 - shadow);
                let base_g = 0x22 as f32 * (1.0 - shadow);
                let base_b = 0x28 as f32 * (1.0 - shadow);
                
                // Subtle texture
                let noise = ((x.wrapping_mul(11) ^ y.wrapping_mul(23)) % 3) as f32;
                let r = (base_r + noise) as u32;
                let g = (base_g + noise) as u32;
                let b = (base_b + noise) as u32;
                
                frame[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
            }
        }
    }
    
    // Front edge highlight (thin light line at very bottom)
    let edge_y = win_h - 2;
    if edge_y * win_w < frame.len() {
        for x in 100..(win_w - 100) {
            let idx = edge_y * win_w + x;
            if idx < frame.len() {
                frame[idx] = 0x3A3A40;
            }
        }
    }
}

/// Read joypad state as a byte (bit layout: A=0, B=1, Select=2, Start=3, Up=4, Down=5, Left=6, Right=7)
fn joypad_to_byte(bus: &Bus, player: u8) -> u8 {
    let jp = if player == 1 { &bus.joypad1 } else { &bus.joypad2 };
    let mut b: u8 = 0;
    if jp.get_button(JoypadButton::A) { b |= 0x01; }
    if jp.get_button(JoypadButton::B) { b |= 0x02; }
    if jp.get_button(JoypadButton::Select) { b |= 0x04; }
    if jp.get_button(JoypadButton::Start) { b |= 0x08; }
    if jp.get_button(JoypadButton::Up) { b |= 0x10; }
    if jp.get_button(JoypadButton::Down) { b |= 0x20; }
    if jp.get_button(JoypadButton::Left) { b |= 0x40; }
    if jp.get_button(JoypadButton::Right) { b |= 0x80; }
    b
}

/// Apply a byte of button state onto a joypad
fn byte_to_joypad(bus: &mut Bus, player: u8, buttons: u8) {
    let jp = if player == 1 { &mut bus.joypad1 } else { &mut bus.joypad2 };
    jp.set_button_pressed(JoypadButton::A, buttons & 0x01 != 0);
    jp.set_button_pressed(JoypadButton::B, buttons & 0x02 != 0);
    jp.set_button_pressed(JoypadButton::Select, buttons & 0x04 != 0);
    jp.set_button_pressed(JoypadButton::Start, buttons & 0x08 != 0);
    jp.set_button_pressed(JoypadButton::Up, buttons & 0x10 != 0);
    jp.set_button_pressed(JoypadButton::Down, buttons & 0x20 != 0);
    jp.set_button_pressed(JoypadButton::Left, buttons & 0x40 != 0);
    jp.set_button_pressed(JoypadButton::Right, buttons & 0x80 != 0);
}

/// Get the recordings directory: ~/.nes-emulator/recordings/
fn recordings_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(|p| PathBuf::from(p).join(".nes-emulator").join("recordings"))
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(|p| PathBuf::from(p).join(".nes-emulator").join("recordings"))
    }
}
