#![windows_subsystem = "windows"]
#![allow(
    clippy::collapsible_match,
    clippy::unnecessary_min_or_max,
    clippy::unnecessary_sort_by
)]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use gilrs::{Axis, Button, Gilrs};
use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use ringbuf::{traits::*, HeapRb};
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use oxidenes::achievements::{md5_hex, AchievementEngine};
use oxidenes::bus::Bus;
use oxidenes::cartridge::Cartridge;
use oxidenes::config::{
    add_recent_game, config_dir, is_favorite, load_config, save_config, toggle_favorite,
    EmulatorConfig, InputBindings,
};
use oxidenes::cpu::Cpu;
use oxidenes::file_browser::FileBrowser;
use oxidenes::joypad::JoypadButton;
use oxidenes::netplay::{NetplaySession, NetplayState};
use oxidenes::ppu::Region;
use oxidenes::recording::{sha256, InputRecording};
use oxidenes::rendering::*;
use oxidenes::rom_library::{
    default_rom_library_dir, import_rom_folder, point_config_at_default_library, RomImportMode,
};
use oxidenes::romdb::RomDatabase;
use oxidenes::scripting::ScriptEngine;
use oxidenes::state_io::StateReader;
use oxidenes::updater::Updater;

// Single source of truth for all screen/window dimensions
const TV_WIDTH: usize = 1240;
const TV_HEIGHT: usize = 884;
const CONSOLE_HEIGHT: usize = 110;
const WINDOW_WIDTH: usize = TV_WIDTH;
const WINDOW_HEIGHT: usize = TV_HEIGHT + CONSOLE_HEIGHT;
const SCREEN_CURVE_SRC_BITS: u32 = 20;
const SCREEN_CURVE_SRC_MASK: u32 = (1 << SCREEN_CURVE_SRC_BITS) - 1;
const SCREEN_CURVE_CORNER_R: usize = 18;

// NES menu colors
const MENU_BG: u32 = 0x0C0C3C;
const MENU_WHITE: u32 = 0xFCFCFC;
const MENU_GOLD: u32 = 0xF8D878;
const MENU_GRAY: u32 = 0x9C9C9C;
const MENU_DARK_GRAY: u32 = 0x585858;
const MENU_LIGHT_BLUE: u32 = 0x6888FC;

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

fn string_to_key(s: &str) -> Option<Key> {
    match s {
        "A" => Some(Key::A),
        "B" => Some(Key::B),
        "C" => Some(Key::C),
        "D" => Some(Key::D),
        "E" => Some(Key::E),
        "F" => Some(Key::F),
        "G" => Some(Key::G),
        "H" => Some(Key::H),
        "I" => Some(Key::I),
        "J" => Some(Key::J),
        "K" => Some(Key::K),
        "L" => Some(Key::L),
        "M" => Some(Key::M),
        "N" => Some(Key::N),
        "O" => Some(Key::O),
        "P" => Some(Key::P),
        "Q" => Some(Key::Q),
        "R" => Some(Key::R),
        "S" => Some(Key::S),
        "T" => Some(Key::T),
        "U" => Some(Key::U),
        "V" => Some(Key::V),
        "W" => Some(Key::W),
        "X" => Some(Key::X),
        "Y" => Some(Key::Y),
        "Z" => Some(Key::Z),
        "Up" => Some(Key::Up),
        "Down" => Some(Key::Down),
        "Left" => Some(Key::Left),
        "Right" => Some(Key::Right),
        "Enter" => Some(Key::Enter),
        "Space" => Some(Key::Space),
        "LeftShift" => Some(Key::LeftShift),
        "RightShift" => Some(Key::RightShift),
        "LeftCtrl" => Some(Key::LeftCtrl),
        "RightCtrl" => Some(Key::RightCtrl),
        "Comma" => Some(Key::Comma),
        "Period" => Some(Key::Period),
        "Slash" => Some(Key::Slash),
        "Semicolon" => Some(Key::Semicolon),
        "Apostrophe" => Some(Key::Apostrophe),
        "1" => Some(Key::Key1),
        "2" => Some(Key::Key2),
        "3" => Some(Key::Key3),
        "4" => Some(Key::Key4),
        "5" => Some(Key::Key5),
        "6" => Some(Key::Key6),
        "7" => Some(Key::Key7),
        "8" => Some(Key::Key8),
        "9" => Some(Key::Key9),
        "0" => Some(Key::Key0),
        "Escape" => Some(Key::Escape),
        "Tab" => Some(Key::Tab),
        "Backspace" => Some(Key::Backspace),
        "Delete" => Some(Key::Delete),
        "Insert" => Some(Key::Insert),
        "Home" => Some(Key::Home),
        "End" => Some(Key::End),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),
        "Pause" => Some(Key::Pause),
        "Menu" => Some(Key::Menu),
        "F1" => Some(Key::F1),
        "F2" => Some(Key::F2),
        "F3" => Some(Key::F3),
        "F4" => Some(Key::F4),
        "F5" => Some(Key::F5),
        "F6" => Some(Key::F6),
        "F7" => Some(Key::F7),
        "F8" => Some(Key::F8),
        "F9" => Some(Key::F9),
        "F10" => Some(Key::F10),
        "F11" => Some(Key::F11),
        "F12" => Some(Key::F12),
        "F13" => Some(Key::F13),
        "F14" => Some(Key::F14),
        "F15" => Some(Key::F15),
        "CapsLock" => Some(Key::CapsLock),
        "NumLock" => Some(Key::NumLock),
        "ScrollLock" => Some(Key::ScrollLock),
        "NumPad0" => Some(Key::NumPad0),
        "NumPad1" => Some(Key::NumPad1),
        "NumPad2" => Some(Key::NumPad2),
        "NumPad3" => Some(Key::NumPad3),
        "NumPad4" => Some(Key::NumPad4),
        "NumPad5" => Some(Key::NumPad5),
        "NumPad6" => Some(Key::NumPad6),
        "NumPad7" => Some(Key::NumPad7),
        "NumPad8" => Some(Key::NumPad8),
        "NumPad9" => Some(Key::NumPad9),
        "NumPadDot" => Some(Key::NumPadDot),
        "NumPadSlash" => Some(Key::NumPadSlash),
        "NumPadAsterisk" => Some(Key::NumPadAsterisk),
        "NumPadMinus" => Some(Key::NumPadMinus),
        "NumPadPlus" => Some(Key::NumPadPlus),
        "NumPadEnter" => Some(Key::NumPadEnter),
        "LeftAlt" => Some(Key::LeftAlt),
        "RightAlt" => Some(Key::RightAlt),
        "LeftSuper" => Some(Key::LeftSuper),
        "RightSuper" => Some(Key::RightSuper),
        "Backquote" => Some(Key::Backquote),
        "Backslash" => Some(Key::Backslash),
        "Equal" => Some(Key::Equal),
        "Minus" => Some(Key::Minus),
        "LeftBracket" => Some(Key::LeftBracket),
        "RightBracket" => Some(Key::RightBracket),
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
        Key::A => "A".to_string(),
        Key::B => "B".to_string(),
        Key::C => "C".to_string(),
        Key::D => "D".to_string(),
        Key::E => "E".to_string(),
        Key::F => "F".to_string(),
        Key::G => "G".to_string(),
        Key::H => "H".to_string(),
        Key::I => "I".to_string(),
        Key::J => "J".to_string(),
        Key::K => "K".to_string(),
        Key::L => "L".to_string(),
        Key::M => "M".to_string(),
        Key::N => "N".to_string(),
        Key::O => "O".to_string(),
        Key::P => "P".to_string(),
        Key::Q => "Q".to_string(),
        Key::R => "R".to_string(),
        Key::S => "S".to_string(),
        Key::T => "T".to_string(),
        Key::U => "U".to_string(),
        Key::V => "V".to_string(),
        Key::W => "W".to_string(),
        Key::X => "X".to_string(),
        Key::Y => "Y".to_string(),
        Key::Z => "Z".to_string(),
        Key::Up => "Up".to_string(),
        Key::Down => "Down".to_string(),
        Key::Left => "Left".to_string(),
        Key::Right => "Right".to_string(),
        Key::Enter => "Enter".to_string(),
        Key::Space => "Space".to_string(),
        Key::LeftShift => "LeftShift".to_string(),
        Key::RightShift => "RightShift".to_string(),
        Key::LeftCtrl => "LeftCtrl".to_string(),
        Key::RightCtrl => "RightCtrl".to_string(),
        Key::Comma => "Comma".to_string(),
        Key::Period => "Period".to_string(),
        Key::Slash => "Slash".to_string(),
        Key::Semicolon => "Semicolon".to_string(),
        Key::Apostrophe => "Apostrophe".to_string(),
        Key::Key1 => "1".to_string(),
        Key::Key2 => "2".to_string(),
        Key::Key3 => "3".to_string(),
        Key::Key4 => "4".to_string(),
        Key::Key5 => "5".to_string(),
        Key::Key6 => "6".to_string(),
        Key::Key7 => "7".to_string(),
        Key::Key8 => "8".to_string(),
        Key::Key9 => "9".to_string(),
        Key::Key0 => "0".to_string(),
        Key::Escape => "Escape".to_string(),
        Key::Tab => "Tab".to_string(),
        Key::Backspace => "Backspace".to_string(),
        Key::Delete => "Delete".to_string(),
        Key::Insert => "Insert".to_string(),
        Key::Home => "Home".to_string(),
        Key::End => "End".to_string(),
        Key::PageUp => "PageUp".to_string(),
        Key::PageDown => "PageDown".to_string(),
        Key::Pause => "Pause".to_string(),
        Key::Menu => "Menu".to_string(),
        Key::F1 => "F1".to_string(),
        Key::F2 => "F2".to_string(),
        Key::F3 => "F3".to_string(),
        Key::F4 => "F4".to_string(),
        Key::F5 => "F5".to_string(),
        Key::F6 => "F6".to_string(),
        Key::F7 => "F7".to_string(),
        Key::F8 => "F8".to_string(),
        Key::F9 => "F9".to_string(),
        Key::F10 => "F10".to_string(),
        Key::F11 => "F11".to_string(),
        Key::F12 => "F12".to_string(),
        Key::F13 => "F13".to_string(),
        Key::F14 => "F14".to_string(),
        Key::F15 => "F15".to_string(),
        Key::CapsLock => "CapsLock".to_string(),
        Key::NumLock => "NumLock".to_string(),
        Key::ScrollLock => "ScrollLock".to_string(),
        Key::NumPad0 => "NumPad0".to_string(),
        Key::NumPad1 => "NumPad1".to_string(),
        Key::NumPad2 => "NumPad2".to_string(),
        Key::NumPad3 => "NumPad3".to_string(),
        Key::NumPad4 => "NumPad4".to_string(),
        Key::NumPad5 => "NumPad5".to_string(),
        Key::NumPad6 => "NumPad6".to_string(),
        Key::NumPad7 => "NumPad7".to_string(),
        Key::NumPad8 => "NumPad8".to_string(),
        Key::NumPad9 => "NumPad9".to_string(),
        Key::NumPadDot => "NumPadDot".to_string(),
        Key::NumPadSlash => "NumPadSlash".to_string(),
        Key::NumPadAsterisk => "NumPadAsterisk".to_string(),
        Key::NumPadMinus => "NumPadMinus".to_string(),
        Key::NumPadPlus => "NumPadPlus".to_string(),
        Key::NumPadEnter => "NumPadEnter".to_string(),
        Key::LeftAlt => "LeftAlt".to_string(),
        Key::RightAlt => "RightAlt".to_string(),
        Key::LeftSuper => "LeftSuper".to_string(),
        Key::RightSuper => "RightSuper".to_string(),
        Key::Backquote => "Backquote".to_string(),
        Key::Backslash => "Backslash".to_string(),
        Key::Equal => "Equal".to_string(),
        Key::Minus => "Minus".to_string(),
        Key::LeftBracket => "LeftBracket".to_string(),
        Key::RightBracket => "RightBracket".to_string(),
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

#[derive(Clone, Copy, PartialEq)]
enum OsdType {
    None,
    Brightness,
    Contrast,
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
    if rom_name.is_empty() {
        return;
    }
    let dir = cheats_dir();
    let _ = fs::create_dir_all(&dir);
    let entries: Vec<serde_json::Value> = cheats
        .iter()
        .map(|c| serde_json::json!({ "code": c.code_str, "enabled": c.enabled }))
        .collect();
    if let Ok(data) = serde_json::to_string_pretty(&entries) {
        let _ = fs::write(dir.join(format!("{}.json", rom_name)), data);
    }
}

fn load_cheats(rom_name: &str) -> Vec<oxidenes::bus::GameGenieCode> {
    if rom_name.is_empty() {
        return Vec::new();
    }
    let path = cheats_dir().join(format!("{}.json", rom_name));
    let Ok(data) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&data) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|e| {
            let code_str = e.get("code")?.as_str()?;
            let enabled = e.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut code = oxidenes::bus::GameGenieCode::decode(code_str)?;
            code.enabled = enabled;
            Some(code)
        })
        .collect()
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
    if data.len() == 64 * 60 * 3 {
        Some(data)
    } else {
        None
    }
}

fn save_state(bus: &Bus, cpu: &Cpu, config: &EmulatorConfig, slot: u8) -> bool {
    let path_opt = save_state_path(config, slot);
    let Some(path) = path_opt else {
        return false;
    };
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
    let Some(path) = path_opt else {
        return false;
    };
    if !path.exists() {
        return false;
    }
    let Ok(data) = fs::read(&path) else {
        return false;
    };

    load_state_from_bytes(bus, cpu, &data)
}

fn parse_save_state_payload(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let header = data.get(0..8)?;
    let is_v03 = header == b"NESSAV03";
    if header != b"NESSAV02" && !is_v03 {
        return None;
    }

    let mut reader = StateReader::new(data.get(8..)?);

    // V03: skip ROM fingerprint (deprecated — validation was too aggressive).
    if is_v03 {
        reader.skip(4)?;
    }

    let cpu_state = reader.read_len_prefixed_u32()?;
    let bus_state = reader.read_len_prefixed_u32()?;
    Some((cpu_state, bus_state))
}

fn load_state_from_bytes(bus: &mut Bus, cpu: &mut Cpu, data: &[u8]) -> bool {
    let Some((cpu_state, bus_state)) = parse_save_state_payload(data) else {
        return false;
    };

    let mut cpu_next = Cpu::new();
    if !cpu_next.load_state(cpu_state) {
        return false;
    }
    if !bus.load_state(bus_state) {
        return false;
    }
    *cpu = cpu_next;

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
            frame_skip: 4, // save every 4th frame (~15 snapshots/sec)
            frame_counter: 0,
        }
    }

    fn push_frame(&mut self, bus: &Bus, cpu: &Cpu) {
        self.frame_counter += 1;
        if !self.frame_counter.is_multiple_of(self.frame_skip) {
            return;
        }

        let estimated_size = 512 + bus.ppu.frame_data.len() * 4; // CPU state + bus state + frame data
        let mut snapshot = Vec::with_capacity(estimated_size);
        let cpu_state = cpu.save_state();
        snapshot.extend_from_slice(&(cpu_state.len() as u32).to_le_bytes());
        snapshot.extend(cpu_state);
        let bus_state = bus.save_state();
        snapshot.extend_from_slice(&(bus_state.len() as u32).to_le_bytes());
        snapshot.extend(bus_state);
        // Store PPU frame buffer for smooth rewind playback
        let frame = &bus.ppu.frame_data;
        snapshot.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        // Bulk copy: reinterpret u32 slice as bytes (safe on same-endian round-trip)
        let frame_bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(frame.as_ptr() as *const u8, frame.len() * 4) };
        snapshot.extend_from_slice(frame_bytes);

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
            self.total_bytes = self.total_bytes.saturating_sub(snapshot.len());
            let mut reader = StateReader::new(&snapshot);
            let Some(cpu_state) = reader.read_len_prefixed_u32() else {
                return false;
            };

            let Some(bus_state) = reader.read_len_prefixed_u32() else {
                return false;
            };

            let mut cpu_next = Cpu::new();
            if !cpu_next.load_state(cpu_state) {
                return false;
            }
            if !bus.load_state(bus_state) {
                return false;
            }
            *cpu = cpu_next;

            // Restore PPU frame buffer for smooth rewind playback
            if reader.remaining() > 0 {
                let Some(frame_len) = reader.read_u32_le().map(|len| len as usize) else {
                    return false;
                };
                let Some(frame_byte_len) = frame_len.checked_mul(4) else {
                    return false;
                };
                let Some(src) = reader.read_bytes(frame_byte_len) else {
                    return false;
                };
                bus.ppu.frame_data.resize(frame_len, 0);
                for (pixel, chunk) in bus.ppu.frame_data.iter_mut().zip(src.chunks_exact(4)) {
                    *pixel = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
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
// Battery SRAM persistence for cartridges that expose save RAM.
// =====================================================================

fn sram_path(config: &EmulatorConfig) -> Option<PathBuf> {
    let recent = config.recent_games.first()?;
    let filename = Path::new(recent).file_stem()?.to_string_lossy().to_string();
    Some(save_state_dir().join(format!("{}.sram", filename)))
}

fn auto_save_sram(bus: &Bus, config: &EmulatorConfig) {
    if !bus.cartridge.has_battery {
        return;
    }
    let Some(path) = sram_path(config) else {
        return;
    };
    let _ = fs::create_dir_all(save_state_dir());
    let sram = bus.get_sram();
    if !sram.is_empty() {
        let _ = fs::write(&path, &sram);
    }
}

fn auto_load_sram(bus: &mut Bus, config: &EmulatorConfig) {
    if !bus.cartridge.has_battery {
        return;
    }
    let Some(path) = sram_path(config) else {
        return;
    };
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
    let data = "P6\n256 240\n255\n".to_string();
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

#[allow(clippy::large_enum_variant)]
enum EmulatorState {
    Menu(MenuState),
    Game,
}

struct MenuState {
    selected: usize,
    submenu: Option<SubMenu>,
    cursor_visible: bool,
    cursor_timer: u32,
    marquee_frame: u32,
    marquee_key: String,
    // Screen transition fade effect
    transition_timer: u8, // counts down from 6 to 0
    transition_out: bool, // true = fading out, false = fading in
    favorites_page: usize,
}

impl MenuState {
    fn new() -> Self {
        Self {
            selected: 0,
            submenu: None,
            cursor_visible: true,
            cursor_timer: 0,
            marquee_frame: 0,
            marquee_key: String::new(),
            transition_timer: 0,
            transition_out: false,
            favorites_page: 0,
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum SubMenu {
    Settings {
        selected: usize,
        value_flash: u8,
    },
    FileBrowser(FileBrowser),
    InputSettings(InputSettingsState),
    CrtSettings {
        selected: usize,
        tables_dirty: bool,
        value_flash: u8,
    },
    FolderSetup {
        browser: FileBrowser,
        from_settings: bool,
    },
}

struct InputSettingsState {
    tab: u8, // 0=KB P1, 1=KB P2, 2=Ctrl P1, 3=Ctrl P2
    selected: usize,
    waiting_for_input: bool,
    bindings: InputBindings, // working copy
    conflict_message: Option<String>,
    conflict_timer: u32,
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
    #[allow(clippy::needless_range_loop)]
    for row in 0..8 {
        let bits = glyph[row];
        let y = py + row;
        if y >= 240 {
            break;
        }
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

const MARQUEE_INITIAL_PAUSE_FRAMES: u32 = 45;
const MARQUEE_STEP_FRAMES: u32 = 8;
const MARQUEE_GAP: &str = "   ";

fn truncate_with_ellipsis_chars(text: &str, max_chars: usize) -> String {
    let len = text.chars().count();
    if len <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let prefix: String = text.chars().take(max_chars - 3).collect();
    format!("{}...", prefix)
}

fn marquee_text(text: &str, max_chars: usize, frame: u32) -> String {
    let chars: Vec<char> = text.chars().collect();
    if max_chars == 0 {
        return String::new();
    }
    if chars.len() <= max_chars {
        return text.to_string();
    }

    let gap: Vec<char> = MARQUEE_GAP.chars().collect();
    let cycle_len = chars.len() + gap.len();
    let offset = if frame < MARQUEE_INITIAL_PAUSE_FRAMES {
        0
    } else {
        ((frame - MARQUEE_INITIAL_PAUSE_FRAMES) / MARQUEE_STEP_FRAMES) as usize % cycle_len
    };

    chars
        .iter()
        .chain(gap.iter())
        .chain(chars.iter())
        .skip(offset)
        .take(max_chars)
        .copied()
        .collect()
}

fn draw_prefixed_name_8x8(
    fb: &mut [u32],
    prefix: &str,
    name: &str,
    tile: (usize, usize),
    name_max_chars: usize,
    color: u32,
    marquee_frame: Option<u32>,
) {
    let (tile_x, tile_y) = tile;
    draw_text_8x8(fb, prefix, tile_x, tile_y, color);
    let display_name = if let Some(frame) = marquee_frame {
        marquee_text(name, name_max_chars, frame)
    } else {
        truncate_with_ellipsis_chars(name, name_max_chars)
    };
    draw_text_8x8(
        fb,
        &display_name,
        tile_x + prefix.chars().count(),
        tile_y,
        color,
    );
}

fn selected_marquee_key(
    menu: &MenuState,
    cfg: &EmulatorConfig,
    favorites_valid: &[bool],
) -> String {
    match &menu.submenu {
        None => {
            let valid_favorites: Vec<&String> = cfg
                .favorite_games
                .iter()
                .enumerate()
                .filter(|(i, _)| favorites_valid.get(*i).copied().unwrap_or(false))
                .map(|(_, p)| p)
                .collect();
            let total_favs = valid_favorites.len();
            let per_page = 5usize;
            let total_pages = if total_favs == 0 {
                0
            } else {
                total_favs.div_ceil(per_page)
            };
            let page = menu.favorites_page.min(total_pages.saturating_sub(1));
            let page_start = page * per_page;
            let page_end = (page_start + per_page).min(total_favs);
            let fav_count = page_end - page_start;

            let mut current_row = 4usize;
            if total_favs > 0 {
                current_row += 1 + fav_count + 1;
            }
            let recent_non_fav: Vec<&String> = cfg
                .recent_games
                .iter()
                .filter(|p| !cfg.favorite_games.contains(p))
                .collect();
            let recent_count = recent_non_fav
                .len()
                .min((23_usize.saturating_sub(current_row)).min(8));

            if menu.selected < fav_count {
                format!("home:fav:{}", valid_favorites[page_start + menu.selected])
            } else if menu.selected < fav_count + recent_count {
                format!("home:recent:{}", recent_non_fav[menu.selected - fav_count])
            } else {
                format!("home:item:{}", menu.selected)
            }
        }
        Some(SubMenu::FileBrowser(browser)) => browser
            .entries
            .get(browser.selected)
            .map(|entry| format!("browser:{}", entry.full_path.display()))
            .unwrap_or_else(|| "browser:empty".to_string()),
        Some(SubMenu::FolderSetup { browser, .. }) => browser
            .entries
            .get(browser.selected)
            .map(|entry| format!("folder:{}", entry.full_path.display()))
            .unwrap_or_else(|| "folder:empty".to_string()),
        Some(SubMenu::Settings { selected, .. }) => format!("settings:{}", selected),
        Some(SubMenu::InputSettings(state)) => format!("input:{}:{}", state.tab, state.selected),
        Some(SubMenu::CrtSettings { selected, .. }) => format!("crt:{}", selected),
    }
}

#[cfg(test)]
mod menu_text_tests {
    use super::*;

    #[test]
    fn marquee_keeps_short_text_static() {
        assert_eq!(marquee_text("CONTRA", 12, 0), "CONTRA");
        assert_eq!(marquee_text("CONTRA", 12, 240), "CONTRA");
    }

    #[test]
    fn marquee_pauses_then_scrolls_selected_long_text() {
        let title = "TEENAGE MUTANT NINJA TURTLES";
        assert_eq!(marquee_text(title, 14, 0), "TEENAGE MUTANT");
        assert_eq!(
            marquee_text(
                title,
                14,
                MARQUEE_INITIAL_PAUSE_FRAMES + MARQUEE_STEP_FRAMES
            ),
            "EENAGE MUTANT "
        );
    }

    #[test]
    fn truncation_is_char_count_based() {
        assert_eq!(truncate_with_ellipsis_chars("ABCDEFGHIJ", 7), "ABCD...");
        assert_eq!(truncate_with_ellipsis_chars("ABCDE", 7), "ABCDE");
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn option_value_rejects_missing_or_option_like_values() {
        let args = vec![
            "oxidenes".to_string(),
            "--import-roms".to_string(),
            "--import-mode".to_string(),
            "copy".to_string(),
        ];

        assert_eq!(option_value(&args, "--import-roms"), None);
        assert_eq!(option_value(&args, "--import-mode"), Some("copy"));
    }
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
        if x % 4 < 2 && y < 240 {
            fb[y * 256 + x] = MENU_DARK_GRAY;
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
fn draw_highlight_bar(
    fb: &mut [u32],
    y_start: usize,
    height: usize,
    x_left: usize,
    x_right: usize,
    color: u32,
) {
    for row in y_start..y_start + height {
        if row >= 240 {
            break;
        }
        for col in x_left..x_right.min(256) {
            fb[row * 256 + col] = color;
        }
    }
}

/// Apply a fade overlay to the framebuffer for screen transitions.
/// fade_level: 0=full brightness, 8=nearly black
#[inline]
fn apply_menu_fade(fb: &mut [u32], width: usize, height: usize, fade_level: u8) {
    if fade_level == 0 {
        return;
    }
    // fade_level 0=full brightness, 8=nearly black
    let brightness = (255u32).saturating_sub(fade_level as u32 * 30); // 255, 225, 195... down to 15
    for pixel in fb[..width * height].iter_mut() {
        let r = (((*pixel >> 16) & 0xFF) * brightness) >> 8;
        let g = (((*pixel >> 8) & 0xFF) * brightness) >> 8;
        let b = ((*pixel & 0xFF) * brightness) >> 8;
        *pixel = (r << 16) | (g << 8) | b;
    }
}

fn render_home_screen(
    fb: &mut [u32],
    menu: &MenuState,
    cfg: &EmulatorConfig,
    cursor_visible: bool,
    favorites_valid: &[bool],
    recents_valid: &[bool],
) {
    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 OXIDENES \x11", 2, MENU_GOLD);
    draw_separator_line(fb, 3);

    let mut current_row: usize = 4;
    let mut item_index: usize = 0;

    // === FAVORITES SECTION ===
    let valid_favorites: Vec<&String> = cfg
        .favorite_games
        .iter()
        .enumerate()
        .filter(|(i, _)| favorites_valid.get(*i).copied().unwrap_or(false))
        .map(|(_, p)| p)
        .collect();
    let total_favs = valid_favorites.len();
    let per_page = 5usize;
    let total_pages = if total_favs == 0 {
        0
    } else {
        total_favs.div_ceil(per_page)
    };
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
            if row >= 24 {
                break;
            }

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
            draw_prefixed_name_8x8(
                fb,
                "\x11 ",
                display_name,
                (3, row),
                24,
                color,
                is_selected.then_some(menu.marquee_frame),
            );

            item_index += 1;
            current_row += 1;
        }
        draw_separator_line(fb, current_row);
        current_row += 1;
    }

    // === RECENT GAMES SECTION ===
    let recent_non_fav: Vec<(usize, &String)> = cfg
        .recent_games
        .iter()
        .enumerate()
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
            #[allow(clippy::needless_range_loop)]
            for i in 0..recent_count {
                let row = current_row;
                if row >= 24 {
                    break;
                }

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
                let display = if is_selected {
                    marquee_text(display_name, 26, menu.marquee_frame)
                } else {
                    truncate_with_ellipsis_chars(display_name, 26)
                };
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

fn render_settings(
    fb: &mut [u32],
    cfg: &EmulatorConfig,
    selected: usize,
    cursor_visible: bool,
    audio_volume: u32,
    glass_intensity: u8,
    value_flash: u8,
) {
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
        if s.len() > 17 {
            format!("...{}", &s[s.len() - 14..])
        } else {
            s
        }
    } else {
        "NOT SET".to_string()
    };

    let settings_items = [
        format!("CRT FILTER: {}", if cfg.crt_enabled { "ON" } else { "OFF" }),
        format!(
            "BARREL DISTORTION: {}",
            if cfg.barrel_distortion { "ON" } else { "OFF" }
        ),
        format!("GLASS INTENSITY: {}%", glass_intensity),
        format!("AUDIO VOLUME: {}%", audio_volume),
        format!(
            "REGION: {}",
            if cfg.region == "pal" { "PAL" } else { "NTSC" }
        ),
        "CRT SETTINGS >".to_string(),
        "INPUT SETTINGS >".to_string(),
        format!(
            "CHECK FOR UPDATES: {}",
            if cfg.check_for_updates { "ON" } else { "OFF" }
        ),
        format!("ROM FOLDER: {}", rom_folder_display),
    ];
    let setting_rows = [7, 9, 11, 13, 15, 17, 19, 21, 23];

    for (i, (item, &row)) in settings_items.iter().zip(setting_rows.iter()).enumerate() {
        let is_flashing = i == selected && value_flash > 0;
        let color = if is_flashing {
            MENU_GOLD
        } else if i == selected {
            MENU_WHITE
        } else {
            MENU_GRAY
        };
        if i == selected && cursor_visible {
            draw_char_8x8(fb, '\x10', 3, row, MENU_WHITE);
        }
        draw_text_8x8(fb, item, 5, row, color);
    }

    draw_separator_line(fb, 25);

    draw_text_centered_8x8(fb, "ENTER/LEFT/RIGHT TO CHANGE", 26, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "ESC TO GO BACK", 27, MENU_DARK_GRAY);
}

fn render_crt_settings(
    fb: &mut [u32],
    cfg: &EmulatorConfig,
    selected: usize,
    cursor_visible: bool,
    value_flash: u8,
) {
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

    let items: [(&str, String); 11] = [
        ("SCANLINES:", format_slider_bar(crt.scanline_intensity)),
        ("PHOSPHOR:", format_slider_bar(crt.phosphor_warmth)),
        ("VIGNETTE:", format_slider_bar(crt.vignette_strength)),
        ("BLUR:", format_slider_bar(crt.blur_amount)),
        ("CURVATURE:", format_slider_bar(crt.curvature_strength)),
        ("GLASS:", format_slider_bar(cfg.glass_intensity)),
        ("MASK:", mask_name.to_string()),
        ("MASK INT:", format_slider_bar(crt.mask_intensity)),
        ("BRIGHTNESS:", format!("{:+}", crt.brightness)),
        ("CONTRAST:", format!("{:+}", crt.contrast)),
        ("BACK", String::new()),
    ];
    let rows = [7, 9, 11, 13, 15, 17, 19, 21, 23];

    for (i, ((label, value), &row)) in items.iter().zip(rows.iter()).enumerate() {
        let is_flashing = i == selected && value_flash > 0;
        let color = if is_flashing {
            MENU_GOLD
        } else if i == selected {
            MENU_WHITE
        } else {
            MENU_GRAY
        };
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
        let color = if i == state.tab as usize {
            MENU_LIGHT_BLUE
        } else {
            MENU_DARK_GRAY
        };
        draw_text_8x8(fb, &format!("[{}]", tab), tab_x, 4, color);
        tab_x += 8;
    }

    draw_separator_line(fb, 5);

    // Binding lists based on active tab
    let current_row = 7;

    match state.tab {
        0 | 1 => {
            // Keyboard bindings
            let bindings = if state.tab == 0 {
                &state.bindings.keyboard_p1
            } else {
                &state.bindings.keyboard_p2
            };
            let binding_names = [
                "UP", "DOWN", "LEFT", "RIGHT", "A", "B", "START", "SELECT", "TURBO A", "TURBO B",
            ];
            let binding_values = [
                &bindings.up,
                &bindings.down,
                &bindings.left,
                &bindings.right,
                &bindings.a,
                &bindings.b,
                &bindings.start,
                &bindings.select,
                &bindings.turbo_a,
                &bindings.turbo_b,
            ];

            for (i, (name, value)) in binding_names.iter().zip(binding_values.iter()).enumerate() {
                let row = current_row + i;
                let color = if i == state.selected {
                    MENU_WHITE
                } else {
                    MENU_GRAY
                };

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
            let bindings = if state.tab == 2 {
                &state.bindings.controller_p1
            } else {
                &state.bindings.controller_p2
            };
            let binding_names = [
                "A", "B", "TURBO A", "TURBO B", "START", "SELECT", "DEADZONE",
            ];
            let binding_values = [
                bindings.a.as_str(),
                bindings.b.as_str(),
                bindings.turbo_a.as_str(),
                bindings.turbo_b.as_str(),
                bindings.start.as_str(),
                bindings.select.as_str(),
                "", // Special case for deadzone
            ];

            for (i, (name, value)) in binding_names.iter().zip(binding_values.iter()).enumerate() {
                let row = current_row + i;
                let color = if i == state.selected {
                    MENU_WHITE
                } else {
                    MENU_GRAY
                };

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
    let s = path.to_string_lossy().to_uppercase().replace('\\', "/");
    if s.len() <= max_chars {
        s
    } else {
        format!("...{}", &s[s.len() - (max_chars - 3)..])
    }
}

fn render_file_browser(
    fb: &mut [u32],
    browser: &FileBrowser,
    cursor_visible: bool,
    cfg: &EmulatorConfig,
    marquee_frame: u32,
) {
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

            let display_name = entry.name.to_uppercase();

            let is_fav = !entry.is_dir && is_favorite(cfg, &entry.full_path.to_string_lossy());

            let prefix = if entry.is_dir {
                "> "
            } else if is_fav {
                "\x11 "
            } else {
                "  "
            };

            if is_selected {
                // Highlight bar
                draw_highlight_bar(fb, row * 8, 8, 20, 236, HIGHLIGHT_BG);
                if cursor_visible {
                    draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
                }
                let color = if entry.is_dir {
                    DIR_COLOR_SEL
                } else {
                    MENU_WHITE
                };

                if !entry.is_dir && entry.size_kb > 0 {
                    let size_str = format!("{}K", entry.size_kb);
                    let size_x = 28 - size_str.len().min(6);
                    // Keep the size fixed while the selected long title scrolls.
                    let max_name_chars = size_x.saturating_sub(3);
                    let prefix_chars = prefix.chars().count();
                    let title_chars = max_name_chars.saturating_sub(prefix_chars);
                    draw_prefixed_name_8x8(
                        fb,
                        prefix,
                        &display_name,
                        (3, row),
                        title_chars,
                        color,
                        Some(marquee_frame),
                    );
                    draw_text_8x8(fb, &size_str, size_x, row, MENU_DARK_GRAY);
                } else {
                    draw_prefixed_name_8x8(
                        fb,
                        prefix,
                        &display_name,
                        (3, row),
                        24,
                        color,
                        Some(marquee_frame),
                    );
                }
            } else {
                let color = if entry.is_dir { DIR_COLOR } else { MENU_GRAY };
                draw_prefixed_name_8x8(fb, prefix, &display_name, (3, row), 24, color, None);
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

fn render_folder_setup(
    fb: &mut [u32],
    browser: &FileBrowser,
    cursor_visible: bool,
    marquee_frame: u32,
) {
    const VISIBLE_ROWS: usize = 14;
    const FIRST_ROW: usize = 9;
    const DIR_COLOR: u32 = 0x5C94FC;
    const DIR_COLOR_SEL: u32 = 0x7CB4FC;
    const HIGHLIGHT_BG: u32 = 0x3C3C8C;

    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

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

            let display_name = entry.name.to_uppercase();
            let prefix = if entry.is_dir { "> " } else { "  " };

            if is_selected {
                draw_highlight_bar(fb, row * 8, 8, 20, 236, HIGHLIGHT_BG);
                if cursor_visible {
                    draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
                }
                let color = if entry.is_dir {
                    DIR_COLOR_SEL
                } else {
                    MENU_DARK_GRAY
                };
                draw_prefixed_name_8x8(
                    fb,
                    prefix,
                    &display_name,
                    (3, row),
                    24,
                    color,
                    Some(marquee_frame),
                );
            } else {
                let color = if entry.is_dir {
                    DIR_COLOR
                } else {
                    MENU_DARK_GRAY
                };
                draw_prefixed_name_8x8(fb, prefix, &display_name, (3, row), 24, color, None);
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
            table[y * SCREEN_W + x] = (v.clamp(0.3, 1.0) * 256.0) as u16;
        }
    }
    table
}

/// Precompute fused scanline × vignette table.
/// `sv_table[dst_y * SCREEN_W + dst_x]` stores `(scan_muls[dst_y % 4] * vig) >> 8`
/// as a `u8`, so the hot loop only does one table load instead of a multiply+shift.
fn build_sv_table(vignette_table: &[u16], scanline_intensity: u8) -> Vec<u8> {
    let si = scanline_intensity as u32;
    let scan_muls: [u32; 4] = [
        255,
        255 - (si * 15 / 100),
        255 - (si * 25 / 100),
        255 - si.min(255) * 55 / 100,
    ];
    let mut sv_table = vec![0u8; SCREEN_W * SCREEN_H];
    for dst_y in 0..SCREEN_H {
        let scan_mul = scan_muls[dst_y % 4];
        let row_base = dst_y * SCREEN_W;
        for dst_x in 0..SCREEN_W {
            let idx = row_base + dst_x;
            let vig = vignette_table[idx] as u32;
            sv_table[idx] = ((scan_mul * vig) >> 8) as u8;
        }
    }
    sv_table
}

/// Immediately rebuild sv_table after a scanline_intensity slider change.
/// Called on every slider tick so the CRT live preview is never stale.
fn apply_scanline_intensity_change(
    sv_table: &mut Vec<u8>,
    vignette_table: &[u16],
    new_intensity: u8,
) {
    *sv_table = build_sv_table(vignette_table, new_intensity);
}

/// Immediately rebuild vignette_table and sv_table after a vignette_strength slider change.
/// Both tables must be rebuilt together because sv_table depends on vignette_table.
fn apply_vignette_strength_change(
    sv_table: &mut Vec<u8>,
    vignette_table: &mut Vec<u16>,
    new_strength: u8,
    scanline_intensity: u8,
) {
    *vignette_table = build_vignette_table_with_strength(new_strength);
    *sv_table = build_sv_table(vignette_table, scanline_intensity);
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

fn build_screen_curve_table() -> Vec<u32> {
    let mut table = Vec::with_capacity(SCREEN_W * SCREEN_H);
    let max_src_idx = (1usize << SCREEN_CURVE_SRC_BITS) - 1;
    debug_assert!(SCREEN_W * SCREEN_H <= max_src_idx);

    for dst_y in 0..SCREEN_H {
        let ny = ((dst_y as f32 + 0.5) / SCREEN_H as f32) * 2.0 - 1.0;
        for dst_x in 0..SCREEN_W {
            let nx = ((dst_x as f32 + 0.5) / SCREEN_W as f32) * 2.0 - 1.0;
            let src_idx = dst_y * SCREEN_W + dst_x;

            let edge = nx.abs().max(ny.abs());
            let edge_t = ((edge - 0.82) / 0.18).clamp(0.0, 1.0);
            let corner_t = (nx.abs() * ny.abs()).powf(1.65);
            let shade =
                if rounded_rect_contains(dst_x, dst_y, SCREEN_W, SCREEN_H, SCREEN_CURVE_CORNER_R) {
                    (256.0 - edge_t * edge_t * 18.0 - corner_t * 10.0).clamp(210.0, 256.0) as u32
                } else {
                    0
                };

            table.push((src_idx as u32) | (shade << SCREEN_CURVE_SRC_BITS));
        }
    }

    table
}

fn build_mask_table(mode: &CrtMaskMode, mask_intensity: u8) -> Vec<(u16, u16, u16)> {
    let intensity = mask_intensity as u32;
    let inv = 100 - intensity;

    // Pre-bake mask value with intensity lerp into a >>8 friendly multiplier.
    // result = (255 * inv + mask_val * intensity) / 100, clamped to 0..=256
    // Per-pixel apply becomes just: (channel * mul) >> 8
    let bake = |mask_val: u8| -> u16 {
        ((255u32 * inv + mask_val as u32 * intensity) / 100).min(256) as u16
    };

    let mut table = vec![(256u16, 256u16, 256u16); SCREEN_W * SCREEN_H];
    match mode {
        CrtMaskMode::Off => {} // all (256,256,256) — no effect
        CrtMaskMode::ShadowMask => {
            // Shadow mask: 3×2 repeating phosphor triads with half-cell row offset
            // Finer pattern — each dot is 1 output pixel wide
            for y in 0..SCREEN_H {
                let row_in_cell = y % 2;
                let col_offset = if (y / 2) % 2 == 0 { 0 } else { 1 };
                for x in 0..SCREEN_W {
                    let col_in_cell = (x + col_offset) % 3;
                    let (r, g, b) = match (row_in_cell, col_in_cell) {
                        (0, 0) => (255, 180, 180), // R bright — off channels at 70%
                        (0, 1) => (180, 255, 180), // G bright
                        (0, 2) => (180, 180, 255), // B bright
                        (1, 0) => (220, 160, 160), // R dim
                        (1, 1) => (160, 220, 160), // G dim
                        (1, 2) => (160, 160, 220), // B dim
                        _ => (200, 200, 200),
                    };
                    table[y * SCREEN_W + x] = (bake(r), bake(g), bake(b));
                }
            }
        }
        CrtMaskMode::ApertureGrille => {
            // Aperture grille: 3-wide vertical RGB stripes (Trinitron style)
            // Each color stripe is exactly 1 output pixel wide
            for y in 0..SCREEN_H {
                for x in 0..SCREEN_W {
                    let (r, g, b) = match x % 3 {
                        0 => (255, 180, 180), // R stripe
                        1 => (180, 255, 180), // G stripe
                        2 => (180, 180, 255), // B stripe
                        _ => unreachable!(),
                    };
                    table[y * SCREEN_W + x] = (bake(r), bake(g), bake(b));
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
                                0 | 1 => (255, 180, 180), // R slot
                                2 | 3 => (180, 255, 180), // G slot
                                4 | 5 => (180, 180, 255), // B slot
                                _ => unreachable!(),
                            }
                        }
                        2 => {
                            // Dark gap row between slot groups
                            (160, 160, 160)
                        }
                        _ => unreachable!(),
                    };
                    table[y * SCREEN_W + x] = (bake(r), bake(g), bake(b));
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
        Self {
            up_held: 0,
            down_held: 0,
            left_held: 0,
            right_held: 0,
        }
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
                (*counter - 20).is_multiple_of(6)
            } else if *counter <= 90 {
                // Medium phase: every 3 frames (~20/sec)
                (*counter - 50).is_multiple_of(3)
            } else {
                // Fast phase: every 2 frames (~30/sec)
                (*counter - 90).is_multiple_of(2)
            }
        } else {
            *counter = 0;
            false
        }
    }

    fn process(
        &mut self,
        raw_up: bool,
        raw_down: bool,
        raw_left: bool,
        raw_right: bool,
    ) -> (bool, bool, bool, bool) {
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

fn poll_menu_input(
    window: &Window,
    gilrs: &mut Option<Gilrs>,
    repeat: &mut RepeatTracker,
    menu_deadzone: f32,
    stick_state: &mut StickState,
) -> MenuInput {
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
        up: false,
        down: false,
        left: false,
        right: false,
        confirm,
        back,
        backspace,
        page_up,
        page_down,
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
            let (s_up, s_down, s_left, s_right) =
                stick_to_dpad(stick_x, stick_y, menu_deadzone, stick_state);
            raw_up |= s_up;
            raw_down |= s_down;
            raw_left |= s_left;
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

fn generate_menu_tone<P: ringbuf::traits::Producer<Item = f32>>(
    producer: &mut P,
    frequency: f32,
    duration_ms: u32,
    volume: f32,
    sample_rate: u32,
) {
    let num_samples = (sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let envelope = if i < num_samples / 10 {
            i as f32 / (num_samples as f32 / 10.0)
        } else {
            1.0 - (i as f32 - num_samples as f32 / 10.0) / (num_samples as f32 * 9.0 / 10.0)
        };
        let sample = if (t * frequency * 2.0 * std::f32::consts::PI).sin() > 0.0 {
            volume
        } else {
            -volume
        };
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

fn play_menu_sound<P: ringbuf::traits::Producer<Item = f32>>(
    producer: &mut P,
    sound: MenuSound,
    sample_rate: u32,
    volume: f32,
) {
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
        (
            GetSystemMetrics(SM_CXSCREEN) as usize,
            GetSystemMetrics(SM_CYSCREEN) as usize,
        )
    }
}

#[cfg(not(windows))]
fn get_screen_resolution() -> (usize, usize) {
    (1920, 1080)
}

fn oxidenes_version() -> &'static str {
    env!("OXIDENES_VERSION")
}

fn version_banner() -> String {
    format!("OxideNES v{}", oxidenes_version())
}

fn cli_version_line() -> String {
    format!("oxidenes {}", oxidenes_version())
}

fn main() {
    // Boost Windows timer resolution for smooth frame pacing
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn timeBeginPeriod(uPeriod: u32) -> u32;
        }
        unsafe {
            timeBeginPeriod(1);
        }
    }

    // CLI flags (handle before any initialization)
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("{}", version_banner());
        println!();
        println!("USAGE:");
        println!("    oxidenes [OPTIONS] [ROM_FILE]");
        println!();
        println!("ARGS:");
        println!(
            "    <ROM_FILE>    Path to a .nes ROM file (optional, opens file browser if omitted)"
        );
        println!();
        println!("OPTIONS:");
        println!("    -h, --help       Show this help message");
        println!("    --version        Show version");
        println!("    --script <FILE>  Load a Lua script on startup");
        println!("    --import-roms <DIR>          Import .nes files into the default ROM library");
        println!("    --import-mode <copy|symlink> Import mode for --import-roms (default: copy)");
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
        println!("{}", cli_version_line());
        std::process::exit(0);
    }
    if args.iter().any(|arg| arg == "--import-roms") {
        run_rom_import_command(&args);
    } else if args.iter().any(|arg| arg == "--import-mode") {
        eprintln!("--import-mode requires --import-roms <DIR>");
        std::process::exit(2);
    }

    let mut config = load_config();
    let romdb = RomDatabase::new();
    let updater = Updater::new();
    if config.check_for_updates {
        updater.check_async();
    }
    let mut update_dismissed = false;

    let mut window = Window::new(
        &version_banner(),
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions {
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create window");

    let target_fps = if config.region == "pal" { 50 } else { 60 };
    let frame_duration_ns: u64 = 1_000_000_000 / target_fps as u64;
    window.set_target_fps(0); // Disabled: custom hybrid pacer handles timing

    // Initialize gamepad support
    let mut gilrs = Gilrs::new().ok();
    if let Some(ref g) = gilrs {
        for (_id, gamepad) in g.gamepads() {
            println!(
                "Controller: {} (connected: {})",
                gamepad.name(),
                gamepad.is_connected()
            );
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
    let mut vignette_table =
        build_vignette_table_with_strength(config.crt_config.vignette_strength);
    // Pre-compute fused scanline×vignette table (avoids per-pixel multiply in hot loop)
    let mut sv_table = build_sv_table(&vignette_table, config.crt_config.scanline_intensity);
    // Pre-compute barrel distortion lookup table (configurable curvature)
    let mut distortion_table =
        build_distortion_table_with_curvature(config.crt_config.curvature_strength);
    let flat_distortion_table = build_flat_distortion_table();
    let screen_curve_table = build_screen_curve_table();
    let glare_table = build_glare_table();
    let glass_thickness_table = build_glass_thickness_table();
    let mut mask_table = build_mask_table(
        &config.crt_config.mask_mode,
        config.crt_config.mask_intensity,
    );
    let mut crt_enabled = config.crt_enabled;
    let mut barrel_distortion = config.barrel_distortion;
    let mut audio_volume = config.audio_volume;
    let mut glass_intensity = config.glass_intensity;
    let mut ca_table = build_ca_table(SCREEN_W, SCREEN_H, glass_intensity);
    let mut ghost_alpha_table = build_ghost_alpha_table(glass_intensity);
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
    let mut osd_type: OsdType = OsdType::None;
    let mut osd_value: i32 = 0;
    let mut osd_timer: u32 = 0;
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
    let mut netplay_port_input: String = "7777".to_string();
    let mut netplay_editing_port: bool = false;

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

    let mut overlay_level = PerfOverlayLevel::Off;
    let mut show_help = false;
    let mut fps_timer = std::time::Instant::now();
    let mut fps_frames: u32 = 0;
    let mut fps_display: String = String::new();
    let mut detail_display: String = String::new();
    let mut perf_snapshot = PerfSnapshot::default();
    let mut detail_tick: u32 = 0;
    let mut frame_start = std::time::Instant::now();

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
                            Path::new(rom_path)
                                .file_stem()
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
                            bus.update_cheats_cache();
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
    let mut favorites_valid: Vec<bool> = config
        .favorite_games
        .iter()
        .map(|p| std::path::Path::new(p.as_str()).exists())
        .collect();
    let mut recents_valid: Vec<bool> = config
        .recent_games
        .iter()
        .map(|p| std::path::Path::new(p.as_str()).exists())
        .collect();

    while window.is_open() {
        let mut next_state: Option<EmulatorState> = None;

        sound_cooldown = sound_cooldown.saturating_sub(1);

        match emulator_state {
            EmulatorState::Menu(ref mut menu) => {
                // Update cursor blink (~500ms at 60fps)
                menu.cursor_timer += 1;
                if menu.cursor_timer >= 30 {
                    menu.cursor_timer = 0;
                    menu.cursor_visible = !menu.cursor_visible;
                }

                let input = poll_menu_input(
                    &window,
                    &mut gilrs,
                    &mut repeat_tracker,
                    config.input_bindings.controller_p1.deadzone,
                    &mut stick_state_menu,
                );

                let mut action: Option<MenuAction> = None;
                let mut input_back_crt = false;

                match menu.submenu {
                    None => {
                        // Compute item layout: favorites, then recent (non-fav), then browse, settings
                        let valid_favorites: Vec<String> = config
                            .favorite_games
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| favorites_valid.get(*i).copied().unwrap_or(false))
                            .map(|(_, p)| p.clone())
                            .collect();
                        let total_favs = valid_favorites.len();
                        let per_page = 5usize;
                        let total_pages = if total_favs == 0 {
                            0
                        } else {
                            total_favs.div_ceil(per_page)
                        };
                        let page = menu.favorites_page.min(total_pages.saturating_sub(1));
                        let page_start = page * per_page;
                        let page_end = (page_start + per_page).min(total_favs);
                        let fav_count = page_end - page_start;
                        let recent_non_fav: Vec<String> = config
                            .recent_games
                            .iter()
                            .filter(|p| !config.favorite_games.contains(p))
                            .cloned()
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
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                sound_cooldown = 3;
                            }
                        }

                        if input.up && menu.selected > 0 {
                            menu.selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                sound_cooldown = 3;
                            }
                        }
                        if input.down && menu.selected < total_items - 1 {
                            menu.selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
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
                                favorites_valid = config
                                    .favorite_games
                                    .iter()
                                    .map(|p| std::path::Path::new(p.as_str()).exists())
                                    .collect();
                                recents_valid = config
                                    .recent_games
                                    .iter()
                                    .map(|p| std::path::Path::new(p.as_str()).exists())
                                    .collect();
                                if added {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                } else {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Back,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    // Adjust selection if item was removed from favorites section
                                    if menu.selected < fav_count && menu.selected > 0 {
                                        menu.selected -= 1;
                                    }
                                    // If page now empty, go back a page
                                    let new_total = config
                                        .favorite_games
                                        .iter()
                                        .filter(|p| std::path::Path::new(p.as_str()).exists())
                                        .count();
                                    let new_pages = if new_total == 0 {
                                        0
                                    } else {
                                        new_total.div_ceil(per_page)
                                    };
                                    if menu.favorites_page >= new_pages && menu.favorites_page > 0 {
                                        menu.favorites_page -= 1;
                                    }
                                }
                            }
                        }
                        if input.confirm {
                            if menu.selected < fav_count {
                                let path = valid_favorites[page_start + menu.selected].clone();
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Confirm,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                action = Some(MenuAction::LoadRom(path));
                            } else if menu.selected < fav_count + recent_count {
                                let path = recent_non_fav[menu.selected - fav_count].clone();
                                if Path::new(&path).exists() {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    action = Some(MenuAction::LoadRom(path));
                                } else {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Error,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                }
                            } else if menu.selected == browse_idx {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Confirm,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                menu.submenu = Some(SubMenu::FileBrowser(FileBrowser::new(
                                    config.rom_directory.as_deref(),
                                )));
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                menu.transition_timer = 6;
                                menu.transition_out = false;
                            } else if menu.selected == settings_idx {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Confirm,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                menu.submenu = Some(SubMenu::Settings {
                                    selected: 0,
                                    value_flash: 0,
                                });
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
                                    if url.starts_with("https://github.com/")
                                        || url.starts_with("https://api.github.com/")
                                    {
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
                    Some(SubMenu::Settings {
                        ref mut selected,
                        ref mut value_flash,
                    }) => {
                        if input.up && *selected > 0 {
                            *selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                sound_cooldown = 3; // skip 3 frames between beeps
                            }
                        }
                        if input.down && *selected < 8 {
                            *selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                sound_cooldown = 3; // skip 3 frames between beeps
                            }
                        }
                        if input.confirm || input.left || input.right {
                            let is_slider_adjust =
                                (input.left || input.right) && (*selected == 2 || *selected == 3);
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
                                    if (input.right || input.confirm) && glass_intensity < 100 {
                                        glass_intensity = (glass_intensity + 5).min(100);
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
                                    if (input.right || input.confirm) && audio_volume < 100 {
                                        audio_volume = (audio_volume + 5).min(100);
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    menu.submenu = Some(SubMenu::CrtSettings {
                                        selected: 0,
                                        tables_dirty: false,
                                        value_flash: 0,
                                    });
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                    menu.transition_timer = 6;
                                    menu.transition_out = false;
                                    return; // Skip the confirm sound below
                                }
                                6 => {
                                    // Open input settings
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    menu.submenu =
                                        Some(SubMenu::InputSettings(InputSettingsState {
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
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
                            play_menu_sound(
                                &mut producer,
                                if is_slider_adjust {
                                    MenuSound::Adjust
                                } else {
                                    MenuSound::Confirm
                                },
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
                        }
                        if input.back {
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Back,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
                            menu.submenu = None;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            menu.transition_timer = 6;
                            menu.transition_out = false;
                        }
                    }
                    Some(SubMenu::CrtSettings {
                        ref mut selected,
                        ref mut tables_dirty,
                        ref mut value_flash,
                    }) => {
                        if input.up && *selected > 0 {
                            *selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                sound_cooldown = 3;
                            }
                        }
                        if input.down && *selected < 8 {
                            *selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                sound_cooldown = 3;
                            }
                        }
                        if input.left || input.right {
                            let delta: i16 = if input.right { 5 } else { -5 };
                            match *selected {
                                0 => {
                                    config.crt_config.scanline_intensity =
                                        (config.crt_config.scanline_intensity as i16 + delta)
                                            .clamp(0, 100)
                                            as u8;
                                    apply_scanline_intensity_change(
                                        &mut sv_table,
                                        &vignette_table,
                                        config.crt_config.scanline_intensity,
                                    );
                                    *tables_dirty = true;
                                }
                                1 => {
                                    config.crt_config.phosphor_warmth =
                                        (config.crt_config.phosphor_warmth as i16 + delta)
                                            .clamp(0, 100)
                                            as u8;
                                    *tables_dirty = true;
                                }
                                2 => {
                                    config.crt_config.vignette_strength =
                                        (config.crt_config.vignette_strength as i16 + delta)
                                            .clamp(0, 100)
                                            as u8;
                                    apply_vignette_strength_change(
                                        &mut sv_table,
                                        &mut vignette_table,
                                        config.crt_config.vignette_strength,
                                        config.crt_config.scanline_intensity,
                                    );
                                    *tables_dirty = true;
                                }
                                3 => {
                                    config.crt_config.blur_amount = (config.crt_config.blur_amount
                                        as i16
                                        + delta)
                                        .clamp(0, 100)
                                        as u8;
                                    *tables_dirty = true;
                                }
                                4 => {
                                    config.crt_config.curvature_strength =
                                        (config.crt_config.curvature_strength as i16 + delta)
                                            .clamp(0, 100)
                                            as u8;
                                    *tables_dirty = true;
                                }
                                5 => {
                                    // Glass intensity (existing field)
                                    glass_intensity =
                                        (glass_intensity as i16 + delta).clamp(0, 100) as u8;
                                    config.glass_intensity = glass_intensity;
                                    ca_table = build_ca_table(SCREEN_W, SCREEN_H, glass_intensity);
                                    ghost_alpha_table = build_ghost_alpha_table(glass_intensity);
                                }
                                6 => {
                                    // Cycle mask mode
                                    config.crt_config.mask_mode = match config.crt_config.mask_mode
                                    {
                                        CrtMaskMode::Off => {
                                            if input.right {
                                                CrtMaskMode::ShadowMask
                                            } else {
                                                CrtMaskMode::SlotMask
                                            }
                                        }
                                        CrtMaskMode::ShadowMask => {
                                            if input.right {
                                                CrtMaskMode::ApertureGrille
                                            } else {
                                                CrtMaskMode::Off
                                            }
                                        }
                                        CrtMaskMode::ApertureGrille => {
                                            if input.right {
                                                CrtMaskMode::SlotMask
                                            } else {
                                                CrtMaskMode::ShadowMask
                                            }
                                        }
                                        CrtMaskMode::SlotMask => {
                                            if input.right {
                                                CrtMaskMode::Off
                                            } else {
                                                CrtMaskMode::ApertureGrille
                                            }
                                        }
                                    };
                                    mask_table = build_mask_table(
                                        &config.crt_config.mask_mode,
                                        config.crt_config.mask_intensity,
                                    );
                                    *tables_dirty = true;
                                }
                                7 => {
                                    config.crt_config.mask_intensity =
                                        (config.crt_config.mask_intensity as i16 + delta)
                                            .clamp(0, 100)
                                            as u8;
                                    mask_table = build_mask_table(
                                        &config.crt_config.mask_mode,
                                        config.crt_config.mask_intensity,
                                    );
                                    *tables_dirty = true;
                                }
                                8 => {
                                    config.crt_config.brightness = (config.crt_config.brightness
                                        as i16
                                        + delta)
                                        .clamp(-50, 50)
                                        as i8;
                                    osd_type = OsdType::Brightness;
                                    osd_value = config.crt_config.brightness as i32;
                                    osd_timer = 120;
                                }
                                9 => {
                                    config.crt_config.contrast =
                                        (config.crt_config.contrast as i16 + delta).clamp(-50, 50)
                                            as i8;
                                    osd_type = OsdType::Contrast;
                                    osd_value = config.crt_config.contrast as i32;
                                    osd_timer = 120;
                                }
                                _ => {}
                            }
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Adjust,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
                            save_config(&config);
                            *value_flash = 8;
                        }
                        if input.confirm && *selected == 8 {
                            // BACK
                            input_back_crt = true;
                        }
                        if input.back || input_back_crt {
                            if *tables_dirty {
                                vignette_table = build_vignette_table_with_strength(
                                    config.crt_config.vignette_strength,
                                );
                                distortion_table = build_distortion_table_with_curvature(
                                    config.crt_config.curvature_strength,
                                );
                                sv_table = build_sv_table(
                                    &vignette_table,
                                    config.crt_config.scanline_intensity,
                                );
                            }
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Back,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
                            menu.submenu = Some(SubMenu::Settings {
                                selected: 5,
                                value_flash: 0,
                            });
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
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                if browser.selected < browser.scroll_offset {
                                    browser.scroll_offset = browser.selected;
                                }
                            }
                            if input.down && browser.selected < count - 1 {
                                browser.selected += 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3; // skip 3 frames between beeps
                                }
                                if browser.selected >= browser.scroll_offset + 20 {
                                    browser.scroll_offset = browser.selected.saturating_sub(19);
                                }
                            }
                            if input.confirm {
                                let entry_is_dir = browser.entries[browser.selected].is_dir;
                                let entry_path =
                                    browser.entries[browser.selected].full_path.clone();
                                if entry_is_dir {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    browser.navigate_to(&entry_path);
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                } else {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    action = Some(MenuAction::LoadRom(
                                        entry_path.to_string_lossy().to_string(),
                                    ));
                                }
                            }
                            if input.favorite {
                                if let Some(entry) = browser.entries.get(browser.selected) {
                                    if !entry.is_dir {
                                        let path_str =
                                            entry.full_path.to_string_lossy().to_string();
                                        let added = toggle_favorite(&mut config, &path_str);
                                        save_config(&config);
                                        favorites_valid = config
                                            .favorite_games
                                            .iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists())
                                            .collect();
                                        recents_valid = config
                                            .recent_games
                                            .iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists())
                                            .collect();
                                        if added {
                                            play_menu_sound(
                                                &mut producer,
                                                MenuSound::Confirm,
                                                actual_sample_rate,
                                                audio_volume as f32 / 100.0,
                                            );
                                        } else {
                                            play_menu_sound(
                                                &mut producer,
                                                MenuSound::Back,
                                                actual_sample_rate,
                                                audio_volume as f32 / 100.0,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        if input.back || input.backspace {
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Back,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
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
                                            [
                                                &state.bindings.keyboard_p1.up,
                                                &state.bindings.keyboard_p1.down,
                                                &state.bindings.keyboard_p1.left,
                                                &state.bindings.keyboard_p1.right,
                                                &state.bindings.keyboard_p1.a,
                                                &state.bindings.keyboard_p1.b,
                                                &state.bindings.keyboard_p1.start,
                                                &state.bindings.keyboard_p1.select,
                                                &state.bindings.keyboard_p1.turbo_a,
                                                &state.bindings.keyboard_p1.turbo_b,
                                            ]
                                        } else {
                                            [
                                                &state.bindings.keyboard_p2.up,
                                                &state.bindings.keyboard_p2.down,
                                                &state.bindings.keyboard_p2.left,
                                                &state.bindings.keyboard_p2.right,
                                                &state.bindings.keyboard_p2.a,
                                                &state.bindings.keyboard_p2.b,
                                                &state.bindings.keyboard_p2.start,
                                                &state.bindings.keyboard_p2.select,
                                                &state.bindings.keyboard_p2.turbo_a,
                                                &state.bindings.keyboard_p2.turbo_b,
                                            ]
                                        };

                                        let binding_names = [
                                            "UP", "DOWN", "LEFT", "RIGHT", "A", "B", "START",
                                            "SELECT", "TURBO A", "TURBO B",
                                        ];
                                        let old_value = binding_refs[state.selected].clone();
                                        let mut conflict_idx: Option<usize> = None;

                                        for (i, &existing_key) in binding_refs.iter().enumerate() {
                                            if i != state.selected && existing_key == &key_string {
                                                state.conflict_message = Some(format!(
                                                    "Swapped with {}",
                                                    binding_names[i]
                                                ));
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
                                                8 => {
                                                    state.bindings.keyboard_p1.turbo_a = key_string
                                                }
                                                9 => {
                                                    state.bindings.keyboard_p1.turbo_b = key_string
                                                }
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
                                                8 => {
                                                    state.bindings.keyboard_p2.turbo_a = key_string
                                                }
                                                9 => {
                                                    state.bindings.keyboard_p2.turbo_b = key_string
                                                }
                                                _ => {}
                                            },
                                            _ => {}
                                        }

                                        // Swap: set conflicting binding to the old value
                                        if let Some(ci) = conflict_idx {
                                            match state.tab {
                                                0 => match ci {
                                                    0 => state.bindings.keyboard_p1.up = old_value,
                                                    1 => {
                                                        state.bindings.keyboard_p1.down = old_value
                                                    }
                                                    2 => {
                                                        state.bindings.keyboard_p1.left = old_value
                                                    }
                                                    3 => {
                                                        state.bindings.keyboard_p1.right = old_value
                                                    }
                                                    4 => state.bindings.keyboard_p1.a = old_value,
                                                    5 => state.bindings.keyboard_p1.b = old_value,
                                                    6 => {
                                                        state.bindings.keyboard_p1.start = old_value
                                                    }
                                                    7 => {
                                                        state.bindings.keyboard_p1.select =
                                                            old_value
                                                    }
                                                    8 => {
                                                        state.bindings.keyboard_p1.turbo_a =
                                                            old_value
                                                    }
                                                    9 => {
                                                        state.bindings.keyboard_p1.turbo_b =
                                                            old_value
                                                    }
                                                    _ => {}
                                                },
                                                1 => match ci {
                                                    0 => state.bindings.keyboard_p2.up = old_value,
                                                    1 => {
                                                        state.bindings.keyboard_p2.down = old_value
                                                    }
                                                    2 => {
                                                        state.bindings.keyboard_p2.left = old_value
                                                    }
                                                    3 => {
                                                        state.bindings.keyboard_p2.right = old_value
                                                    }
                                                    4 => state.bindings.keyboard_p2.a = old_value,
                                                    5 => state.bindings.keyboard_p2.b = old_value,
                                                    6 => {
                                                        state.bindings.keyboard_p2.start = old_value
                                                    }
                                                    7 => {
                                                        state.bindings.keyboard_p2.select =
                                                            old_value
                                                    }
                                                    8 => {
                                                        state.bindings.keyboard_p2.turbo_a =
                                                            old_value
                                                    }
                                                    9 => {
                                                        state.bindings.keyboard_p2.turbo_b =
                                                            old_value
                                                    }
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
                                        if let gilrs::EventType::ButtonPressed(btn, _) = event.event
                                        {
                                            let button_string = gilrs_button_to_string(btn);

                                            // Check for conflicts within the same controller
                                            let binding_refs = if state.tab == 2 {
                                                [
                                                    &state.bindings.controller_p1.a,
                                                    &state.bindings.controller_p1.b,
                                                    &state.bindings.controller_p1.turbo_a,
                                                    &state.bindings.controller_p1.turbo_b,
                                                    &state.bindings.controller_p1.start,
                                                    &state.bindings.controller_p1.select,
                                                ]
                                            } else {
                                                [
                                                    &state.bindings.controller_p2.a,
                                                    &state.bindings.controller_p2.b,
                                                    &state.bindings.controller_p2.turbo_a,
                                                    &state.bindings.controller_p2.turbo_b,
                                                    &state.bindings.controller_p2.start,
                                                    &state.bindings.controller_p2.select,
                                                ]
                                            };

                                            let binding_names =
                                                ["A", "B", "TURBO A", "TURBO B", "START", "SELECT"];
                                            let old_value = binding_refs[state.selected].clone();
                                            let mut conflict_idx: Option<usize> = None;

                                            for (i, &existing) in binding_refs.iter().enumerate() {
                                                if i != state.selected && existing == &button_string
                                                {
                                                    state.conflict_message = Some(format!(
                                                        "Swapped with {}",
                                                        binding_names[i]
                                                    ));
                                                    state.conflict_timer = 90;
                                                    conflict_idx = Some(i);
                                                    break;
                                                }
                                            }

                                            // Apply the binding
                                            match state.tab {
                                                2 => match state.selected {
                                                    0 => {
                                                        state.bindings.controller_p1.a =
                                                            button_string
                                                    }
                                                    1 => {
                                                        state.bindings.controller_p1.b =
                                                            button_string
                                                    }
                                                    2 => {
                                                        state.bindings.controller_p1.turbo_a =
                                                            button_string
                                                    }
                                                    3 => {
                                                        state.bindings.controller_p1.turbo_b =
                                                            button_string
                                                    }
                                                    4 => {
                                                        state.bindings.controller_p1.start =
                                                            button_string
                                                    }
                                                    5 => {
                                                        state.bindings.controller_p1.select =
                                                            button_string
                                                    }
                                                    _ => {}
                                                },
                                                3 => match state.selected {
                                                    0 => {
                                                        state.bindings.controller_p2.a =
                                                            button_string
                                                    }
                                                    1 => {
                                                        state.bindings.controller_p2.b =
                                                            button_string
                                                    }
                                                    2 => {
                                                        state.bindings.controller_p2.turbo_a =
                                                            button_string
                                                    }
                                                    3 => {
                                                        state.bindings.controller_p2.turbo_b =
                                                            button_string
                                                    }
                                                    4 => {
                                                        state.bindings.controller_p2.start =
                                                            button_string
                                                    }
                                                    5 => {
                                                        state.bindings.controller_p2.select =
                                                            button_string
                                                    }
                                                    _ => {}
                                                },
                                                _ => {}
                                            }

                                            // Swap: set conflicting binding to the old value
                                            if let Some(ci) = conflict_idx {
                                                match state.tab {
                                                    2 => match ci {
                                                        0 => {
                                                            state.bindings.controller_p1.a =
                                                                old_value
                                                        }
                                                        1 => {
                                                            state.bindings.controller_p1.b =
                                                                old_value
                                                        }
                                                        2 => {
                                                            state.bindings.controller_p1.turbo_a =
                                                                old_value
                                                        }
                                                        3 => {
                                                            state.bindings.controller_p1.turbo_b =
                                                                old_value
                                                        }
                                                        4 => {
                                                            state.bindings.controller_p1.start =
                                                                old_value
                                                        }
                                                        5 => {
                                                            state.bindings.controller_p1.select =
                                                                old_value
                                                        }
                                                        _ => {}
                                                    },
                                                    3 => match ci {
                                                        0 => {
                                                            state.bindings.controller_p2.a =
                                                                old_value
                                                        }
                                                        1 => {
                                                            state.bindings.controller_p2.b =
                                                                old_value
                                                        }
                                                        2 => {
                                                            state.bindings.controller_p2.turbo_a =
                                                                old_value
                                                        }
                                                        3 => {
                                                            state.bindings.controller_p2.turbo_b =
                                                                old_value
                                                        }
                                                        4 => {
                                                            state.bindings.controller_p2.start =
                                                                old_value
                                                        }
                                                        5 => {
                                                            state.bindings.controller_p2.select =
                                                                old_value
                                                        }
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
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Confirm,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                            }

                            if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                                // Cancel capture
                                state.waiting_for_input = false;
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Back,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                            }
                        } else {
                            // Normal navigation mode
                            let max_items = if state.tab < 2 { 9 } else { 6 }; // 10 keyboard items (0-9), 7 controller items (0-6, including deadzone)

                            if input.up && state.selected > 0 {
                                state.selected -= 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            if input.down && state.selected < max_items {
                                state.selected += 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            if input.right && state.tab < 3 && !deadzone_active {
                                state.tab += 1;
                                state.selected = 0;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }

                            if input.confirm {
                                // Start rebinding process
                                if state.tab >= 2 && state.selected == 6 {
                                    // Special case for deadzone - adjust with left/right instead of rebinding
                                } else {
                                    state.waiting_for_input = true;
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                }
                            }

                            // Handle deadzone adjustment for controller tabs
                            if state.tab >= 2 && state.selected == 6 && (input.left || input.right)
                            {
                                let deadzone = if state.tab == 2 {
                                    &mut state.bindings.controller_p1.deadzone
                                } else {
                                    &mut state.bindings.controller_p2.deadzone
                                };
                                if input.left {
                                    *deadzone = (*deadzone - 0.05).max(0.10);
                                }
                                if input.right {
                                    *deadzone = (*deadzone + 0.05).min(0.80);
                                }
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }

                            if input.back {
                                // Save and go back
                                config.input_bindings = state.bindings.clone();
                                save_config(&config);
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Back,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                menu.submenu = Some(SubMenu::Settings {
                                    selected: 4,
                                    value_flash: 0,
                                }); // Return to settings, INPUT SETTINGS selected
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                menu.transition_timer = 6;
                                menu.transition_out = false;
                            }
                        }
                    }
                    Some(SubMenu::FolderSetup {
                        ref mut browser,
                        from_settings,
                    }) => {
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                                if browser.selected >= browser.scroll_offset + 14 {
                                    browser.scroll_offset = browser.selected.saturating_sub(13);
                                }
                            }
                            if input.confirm {
                                let entry_is_dir = browser.entries[browser.selected].is_dir;
                                let entry_path =
                                    browser.entries[browser.selected].full_path.clone();
                                if entry_is_dir {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
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
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Back,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
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
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Confirm,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
                            if fs_from_settings {
                                menu.submenu = Some(SubMenu::Settings {
                                    selected: 8,
                                    value_flash: 0,
                                });
                            } else {
                                menu.submenu = None;
                            }
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            menu.transition_timer = 6;
                            menu.transition_out = false;
                        } else if folder_action == 2 {
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Back,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
                            menu.submenu = Some(SubMenu::Settings {
                                selected: 8,
                                value_flash: 0,
                            });
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
                                draw_text_centered_8x8(
                                    &mut menu_framebuffer,
                                    "LOADING...",
                                    15,
                                    0xF8D878,
                                );
                                let dt = if barrel_distortion {
                                    &distortion_table
                                } else {
                                    &flat_distortion_table
                                };
                                if crt_enabled {
                                    crt_filter(
                                        &menu_framebuffer,
                                        &mut crt_buffer,
                                        &sv_table,
                                        dt,
                                        &config.crt_config,
                                        &mask_table,
                                        config.crt_config.brightness as i32,
                                        config.crt_config.contrast as i32,
                                    );
                                    // Phosphor bloom — bright pixels glow into neighbors
                                    apply_phosphor_bloom(
                                        &mut crt_buffer,
                                        SCREEN_W,
                                        SCREEN_H,
                                        config.crt_config.phosphor_warmth as u32,
                                    );
                                    apply_scanline_glow(
                                        &mut crt_buffer,
                                        SCREEN_W,
                                        SCREEN_H,
                                        config.crt_config.phosphor_warmth as u32,
                                    );
                                } else {
                                    scale_simple(&menu_framebuffer, &mut crt_buffer);
                                }
                                composite_screen_fast(
                                    &mut composite_buffer,
                                    &crt_buffer,
                                    &screen_curve_table,
                                    WINDOW_WIDTH,
                                );
                                let _ = window.update_with_buffer(
                                    &composite_buffer,
                                    WINDOW_WIDTH,
                                    WINDOW_HEIGHT,
                                );

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
                                        favorites_valid = config
                                            .favorite_games
                                            .iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists())
                                            .collect();
                                        recents_valid = config
                                            .recent_games
                                            .iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists())
                                            .collect();
                                        auto_load_sram(&mut bus, &config);
                                        rewind_buffer.clear();
                                        game_bus = Some(bus);
                                        game_cpu = Some(cpu);
                                        next_state = Some(EmulatorState::Game);
                                        println!("Loaded: {}", path_str);
                                        let game_name = rom_title.unwrap_or_else(|| {
                                            Path::new(&path_str)
                                                .file_stem()
                                                .map(|s| s.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "Unknown".to_string())
                                        });
                                        window_title = format!("OxideNES — {}", game_name);
                                        window.set_title(&window_title);
                                        // Initialize achievements & recording for this ROM
                                        let rom_md5 = md5_hex(&rom_data);
                                        achievement_engine =
                                            AchievementEngine::load_for_rom(&rom_md5);
                                        let rom_sha = sha256(&rom_data);
                                        recorder = InputRecording::new(rom_sha);
                                        current_rom_name = game_name;
                                        current_rom_path = path_str.clone();
                                        // Load persisted Game Genie cheats for this ROM
                                        if let Some(ref mut bus) = game_bus {
                                            bus.cheats = load_cheats(&current_rom_name);
                                            bus.update_cheats_cache();
                                        }
                                    }
                                    Err(e) => {
                                        let msg = e.to_string();
                                        if let Some(SubMenu::FileBrowser(ref mut browser)) =
                                            menu.submenu
                                        {
                                            browser.error_message = Some(msg.clone());
                                            browser.error_timer = 180;
                                        }
                                        eprintln!("ROM Error: {}", msg);
                                    }
                                }
                            }
                            Err(e) => {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Error,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
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

                let marquee_key = selected_marquee_key(menu, &config, &favorites_valid);
                if marquee_key == menu.marquee_key {
                    menu.marquee_frame = menu.marquee_frame.wrapping_add(1);
                } else {
                    menu.marquee_key = marquee_key;
                    menu.marquee_frame = 0;
                }

                // Render menu to 256x240 framebuffer
                match menu.submenu {
                    None => {
                        render_home_screen(
                            &mut menu_framebuffer,
                            menu,
                            &config,
                            menu.cursor_visible,
                            &favorites_valid,
                            &recents_valid,
                        );
                        // Show update banner if available and not dismissed
                        if !update_dismissed {
                            if let Some(info) = updater.get_update() {
                                let banner = format!("v{}", info.version);
                                let banner_x = 32 - banner.len() - 1;
                                draw_text_8x8(
                                    &mut menu_framebuffer,
                                    &banner,
                                    banner_x,
                                    29,
                                    MENU_DARK_GRAY,
                                );
                            }
                        }
                    }
                    Some(SubMenu::Settings {
                        selected,
                        ref mut value_flash,
                    }) => {
                        render_settings(
                            &mut menu_framebuffer,
                            &config,
                            selected,
                            menu.cursor_visible,
                            audio_volume,
                            glass_intensity,
                            *value_flash,
                        );
                        if *value_flash > 0 {
                            *value_flash -= 1;
                        }
                    }
                    Some(SubMenu::FileBrowser(ref browser)) => {
                        render_file_browser(
                            &mut menu_framebuffer,
                            browser,
                            menu.cursor_visible,
                            &config,
                            menu.marquee_frame,
                        );
                    }
                    Some(SubMenu::InputSettings(ref state)) => {
                        render_input_settings(&mut menu_framebuffer, state, menu.cursor_visible);
                    }
                    Some(SubMenu::CrtSettings {
                        selected,
                        ref mut value_flash,
                        ..
                    }) => {
                        render_crt_settings(
                            &mut menu_framebuffer,
                            &config,
                            selected,
                            menu.cursor_visible,
                            *value_flash,
                        );
                        if *value_flash > 0 {
                            *value_flash -= 1;
                        }
                    }
                    Some(SubMenu::FolderSetup { ref browser, .. }) => {
                        render_folder_setup(
                            &mut menu_framebuffer,
                            browser,
                            menu.cursor_visible,
                            menu.marquee_frame,
                        );
                    }
                }

                // Apply screen transition fade
                if menu.transition_timer > 0 {
                    apply_menu_fade(&mut menu_framebuffer, 256, 240, menu.transition_timer);
                    menu.transition_timer -= 1;
                }

                // Apply CRT filter pipeline (same as game!)
                let dt = if barrel_distortion {
                    &distortion_table
                } else {
                    &flat_distortion_table
                };
                if crt_enabled {
                    crt_filter(
                        &menu_framebuffer,
                        &mut crt_buffer,
                        &sv_table,
                        dt,
                        &config.crt_config,
                        &mask_table,
                        config.crt_config.brightness as i32,
                        config.crt_config.contrast as i32,
                    );
                    // Phosphor bloom — bright pixels glow into neighbors
                    apply_phosphor_bloom(
                        &mut crt_buffer,
                        SCREEN_W,
                        SCREEN_H,
                        config.crt_config.phosphor_warmth as u32,
                    );
                    apply_scanline_glow(
                        &mut crt_buffer,
                        SCREEN_W,
                        SCREEN_H,
                        config.crt_config.phosphor_warmth as u32,
                    );
                    // Apply chromatic aberration to crt_buffer (screen area only)
                    if glass_intensity > 30 {
                        apply_chromatic_aberration(
                            &mut ca_temp,
                            &crt_buffer,
                            &ca_table,
                            SCREEN_W,
                            SCREEN_H,
                        );
                        std::mem::swap(&mut crt_buffer, &mut ca_temp);
                    }
                } else {
                    scale_simple(&menu_framebuffer, &mut crt_buffer);
                }
                composite_screen_fast(
                    &mut composite_buffer,
                    &crt_buffer,
                    &screen_curve_table,
                    WINDOW_WIDTH,
                );
                if crt_enabled && glass_intensity > 0 {
                    apply_glass_effects(
                        &mut composite_buffer,
                        &crt_buffer,
                        &glare_table,
                        &glass_thickness_table,
                        &ghost_alpha_table,
                        WINDOW_WIDTH,
                        glass_intensity,
                        false,
                        SCREEN_W,
                    );
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
                                    Key::Backspace => {
                                        cheat_input_buffer.pop();
                                    }
                                    Key::Enter => {
                                        if cheat_input_buffer.len() == 6
                                            || cheat_input_buffer.len() == 8
                                        {
                                            if let Some(code) = oxidenes::bus::GameGenieCode::decode(
                                                &cheat_input_buffer,
                                            ) {
                                                bus.cheats.push(code);
                                                bus.update_cheats_cache();
                                                save_cheats(&current_rom_name, &bus.cheats);
                                                cheat_message =
                                                    Some(format!("ADDED: {}", cheat_input_buffer));
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
                            let input = poll_menu_input(
                                &window,
                                &mut gilrs,
                                &mut repeat_tracker,
                                config.input_bindings.controller_p1.deadzone,
                                &mut stick_state_menu,
                            );
                            // Items: each cheat (toggle), then ADD CODE, CLEAR ALL
                            let item_count = bus.cheats.len() + 2;
                            if input.up && cheats_selected > 0 {
                                cheats_selected -= 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            if input.down && cheats_selected < item_count - 1 {
                                cheats_selected += 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            if input.confirm {
                                if cheats_selected < bus.cheats.len() {
                                    // Toggle cheat on/off
                                    bus.cheats[cheats_selected].enabled =
                                        !bus.cheats[cheats_selected].enabled;
                                    bus.update_cheats_cache();
                                    save_cheats(&current_rom_name, &bus.cheats);
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                } else if cheats_selected == bus.cheats.len() {
                                    // ADD CODE
                                    cheat_input_mode = true;
                                    cheat_input_buffer.clear();
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                } else {
                                    // CLEAR ALL
                                    bus.cheats.clear();
                                    bus.update_cheats_cache();
                                    save_cheats(&current_rom_name, &bus.cheats);
                                    cheats_selected = 0;
                                    cheat_message = Some("ALL CHEATS CLEARED".to_string());
                                    cheat_message_timer = 90;
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                }
                            }
                            if input.backspace && cheats_selected < bus.cheats.len() {
                                // Delete individual cheat with Backspace
                                bus.cheats.remove(cheats_selected);
                                bus.update_cheats_cache();
                                save_cheats(&current_rom_name, &bus.cheats);
                                if cheats_selected >= bus.cheats.len() + 2 {
                                    cheats_selected = (bus.cheats.len() + 1).max(0);
                                }
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Confirm,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                            }
                            if input.back {
                                cheats_submenu = false;
                                cheat_input_mode = false;
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Back,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                            }
                        }
                    } else if paused && controls_submenu {
                        // Controls reference page input handling
                        let input = poll_menu_input(
                            &window,
                            &mut gilrs,
                            &mut repeat_tracker,
                            config.input_bindings.controller_p1.deadzone,
                            &mut stick_state_menu,
                        );
                        if input.back {
                            controls_submenu = false;
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Back,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
                        }
                    } else if paused && achievement_submenu {
                        // Achievement submenu input handling
                        let input = poll_menu_input(
                            &window,
                            &mut gilrs,
                            &mut repeat_tracker,
                            config.input_bindings.controller_p1.deadzone,
                            &mut stick_state_menu,
                        );
                        if input.back {
                            achievement_submenu = false;
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Back,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
                        }
                    } else if paused && netplay_submenu {
                        // Netplay submenu input handling
                        if netplay_editing_port {
                            // Port text input mode
                            for key in &window.get_keys_pressed(KeyRepeat::Yes) {
                                match key {
                                    Key::Key0 | Key::NumPad0 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('0');
                                        }
                                    }
                                    Key::Key1 | Key::NumPad1 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('1');
                                        }
                                    }
                                    Key::Key2 | Key::NumPad2 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('2');
                                        }
                                    }
                                    Key::Key3 | Key::NumPad3 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('3');
                                        }
                                    }
                                    Key::Key4 | Key::NumPad4 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('4');
                                        }
                                    }
                                    Key::Key5 | Key::NumPad5 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('5');
                                        }
                                    }
                                    Key::Key6 | Key::NumPad6 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('6');
                                        }
                                    }
                                    Key::Key7 | Key::NumPad7 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('7');
                                        }
                                    }
                                    Key::Key8 | Key::NumPad8 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('8');
                                        }
                                    }
                                    Key::Key9 | Key::NumPad9 => {
                                        if netplay_port_input.len() < 5 {
                                            netplay_port_input.push('9');
                                        }
                                    }
                                    Key::Backspace => {
                                        netplay_port_input.pop();
                                    }
                                    Key::Enter => {
                                        if let Ok(p) = netplay_port_input.parse::<u16>() {
                                            netplay.port = p;
                                            // Sync port in join address
                                            if let Some(colon_pos) = netplay_ip_input.rfind(':') {
                                                netplay_ip_input = format!(
                                                    "{}:{}",
                                                    &netplay_ip_input[..colon_pos],
                                                    p
                                                );
                                            } else {
                                                netplay_ip_input =
                                                    format!("{}:{}", netplay_ip_input, p);
                                            }
                                        } else {
                                            // Invalid port, restore from current
                                            netplay_port_input = format!("{}", netplay.port);
                                        }
                                        netplay_editing_port = false;
                                    }
                                    Key::Escape => {
                                        netplay_port_input = format!("{}", netplay.port);
                                        netplay_editing_port = false;
                                    }
                                    _ => {}
                                }
                            }
                        } else if netplay_ip_editing {
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
                                    Key::Backspace => {
                                        netplay_ip_input.pop();
                                    }
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
                            let input = poll_menu_input(
                                &window,
                                &mut gilrs,
                                &mut repeat_tracker,
                                config.input_bindings.controller_p1.deadzone,
                                &mut stick_state_menu,
                            );
                            if input.up && netplay_selected > 0 {
                                netplay_selected -= 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            if input.down && netplay_selected < 4 {
                                netplay_selected += 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            if input.confirm {
                                match netplay_selected {
                                    0 => {
                                        // Port
                                        netplay_editing_port = true;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    1 => {
                                        // Host
                                        if let Ok(p) = netplay_port_input.parse::<u16>() {
                                            netplay.port = p;
                                            match netplay.host() {
                                                Ok(()) => {
                                                    overlay_message = Some(format!(
                                                        "HOSTING ON PORT {}",
                                                        netplay.port
                                                    ));
                                                    overlay_timer = 120;
                                                    netplay_submenu = false;
                                                    paused = false;
                                                }
                                                Err(e) => {
                                                    overlay_message =
                                                        Some(format!("HOST FAILED: {}", e));
                                                    overlay_timer = 120;
                                                }
                                            }
                                        } else {
                                            overlay_message = Some("INVALID PORT".to_string());
                                            overlay_timer = 120;
                                        }
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    2 => {
                                        // Join
                                        netplay_ip_editing = true;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    3 => {
                                        // Disconnect
                                        netplay.disconnect();
                                        overlay_message = Some("NETPLAY DISCONNECTED".to_string());
                                        overlay_timer = 90;
                                        netplay_submenu = false;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    4 => {
                                        // Input delay toggle (cycle 1-5)
                                        netplay.input_delay = if netplay.input_delay >= 5 {
                                            1
                                        } else {
                                            netplay.input_delay + 1
                                        };
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Cursor,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                            if input.back {
                                netplay_submenu = false;
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Back,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                            }
                        }
                    } else if paused {
                        let input = poll_menu_input(
                            &window,
                            &mut gilrs,
                            &mut repeat_tracker,
                            config.input_bindings.controller_p1.deadzone,
                            &mut stick_state_menu,
                        );
                        if input.up && pause_selected > 0 {
                            pause_selected -= 1;
                            if sound_cooldown == 0 {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                sound_cooldown = 3;
                            }
                        }
                        if input.down && pause_selected < 13 {
                            pause_selected += 1;
                            if sound_cooldown == 0 {
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Cursor,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                                sound_cooldown = 3;
                            }
                        }
                        // L/R cycles save slot when on Save or Load items
                        if pause_selected == 1 || pause_selected == 2 {
                            if input.left {
                                current_save_slot = if current_save_slot == 1 {
                                    5
                                } else {
                                    current_save_slot - 1
                                };
                                pause_save_label =
                                    format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                                pause_load_label =
                                    format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            if input.right {
                                current_save_slot = if current_save_slot == 5 {
                                    1
                                } else {
                                    current_save_slot + 1
                                };
                                pause_save_label =
                                    format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                                pause_load_label =
                                    format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                        }
                        if input.confirm {
                            match pause_selected {
                                0 => {
                                    // Resume
                                    paused = false;
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                }
                                1 => {
                                    // Save state
                                    if save_state(bus, cpu, &config, current_save_slot) {
                                        thumbnail_cache[(current_save_slot as usize)
                                            .saturating_sub(1)
                                            .min(3)] = load_thumbnail(&config, current_save_slot);
                                        overlay_message = Some("STATE SAVED".to_string());
                                        overlay_timer = 90;
                                        paused = false;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    } else {
                                        overlay_message = Some("NO SRAM FOUND".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Error,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                }
                                2 => {
                                    // Load state
                                    if load_state(bus, cpu, &config, current_save_slot) {
                                        overlay_message = Some("STATE LOADED".to_string());
                                        overlay_timer = 90;
                                        paused = false;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    } else {
                                        overlay_message = Some("NO SAVE FOUND".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Error,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                }
                                3 => {
                                    // Cheats submenu
                                    cheats_submenu = true;
                                    cheats_selected = 0;
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                }
                                4 => {
                                    // Netplay
                                    netplay_submenu = true;
                                    netplay_selected = 0;
                                    netplay_ip_editing = false;
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                }
                                5 => {
                                    // Reload / Load script
                                    let path = script_engine
                                        .as_ref()
                                        .and_then(|s| s.script_path.clone())
                                        .or_else(|| script_path_arg.clone());
                                    if let Some(spath) = path {
                                        let mut engine = ScriptEngine::init();
                                        match engine.load_script(&spath) {
                                            Ok(()) => {
                                                script_engine = Some(engine);
                                                overlay_message = Some("SCRIPT LOADED".to_string());
                                                overlay_timer = 90;
                                                play_menu_sound(
                                                    &mut producer,
                                                    MenuSound::Confirm,
                                                    actual_sample_rate,
                                                    audio_volume as f32 / 100.0,
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("[scripting] {}", e);
                                                overlay_message = Some("SCRIPT ERROR".to_string());
                                                overlay_timer = 90;
                                                play_menu_sound(
                                                    &mut producer,
                                                    MenuSound::Error,
                                                    actual_sample_rate,
                                                    audio_volume as f32 / 100.0,
                                                );
                                            }
                                        }
                                    } else {
                                        overlay_message =
                                            Some("NO SCRIPT SET (--script)".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Error,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    paused = false;
                                }
                                6 => {
                                    // Unload script
                                    if let Some(ref mut engine) = script_engine {
                                        engine.unload();
                                        overlay_message = Some("SCRIPT UNLOADED".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    script_engine = None;
                                    paused = false;
                                }
                                7 => {
                                    // Toggle favorite
                                    if !current_rom_path.is_empty() {
                                        let added = toggle_favorite(&mut config, &current_rom_path);
                                        save_config(&config);
                                        favorites_valid = config
                                            .favorite_games
                                            .iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists())
                                            .collect();
                                        recents_valid = config
                                            .recent_games
                                            .iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists())
                                            .collect();
                                        if added {
                                            overlay_message =
                                                Some("ADDED TO FAVORITES".to_string());
                                            play_menu_sound(
                                                &mut producer,
                                                MenuSound::Confirm,
                                                actual_sample_rate,
                                                audio_volume as f32 / 100.0,
                                            );
                                        } else {
                                            overlay_message =
                                                Some("REMOVED FROM FAVORITES".to_string());
                                            play_menu_sound(
                                                &mut producer,
                                                MenuSound::Back,
                                                actual_sample_rate,
                                                audio_volume as f32 / 100.0,
                                            );
                                        }
                                        overlay_timer = 90;
                                    }
                                }
                                8 => {
                                    // Return to menu
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
                                    window_title =
                                        format!("OxideNES v{}", env!("OXIDENES_VERSION"));
                                    window.set_title(&window_title);
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Back,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    continue;
                                }
                                9 => {
                                    // Achievements
                                    achievement_submenu = !achievement_submenu;
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                }
                                10 => {
                                    // Save recording
                                    if recorder.frame_count() > 0 {
                                        if let Some(base) = recordings_dir() {
                                            let _ = std::fs::create_dir_all(&base);
                                            let path =
                                                base.join(format!("{}.nrec", current_rom_name));
                                            match recorder.save_to_file(
                                                path.to_str().unwrap_or("recording.nrec"),
                                            ) {
                                                Ok(()) => {
                                                    overlay_message =
                                                        Some("RECORDING SAVED".to_string());
                                                    overlay_timer = 90;
                                                    play_menu_sound(
                                                        &mut producer,
                                                        MenuSound::Confirm,
                                                        actual_sample_rate,
                                                        audio_volume as f32 / 100.0,
                                                    );
                                                }
                                                Err(e) => {
                                                    eprintln!("[recording] Save error: {}", e);
                                                    overlay_message =
                                                        Some("SAVE FAILED".to_string());
                                                    overlay_timer = 90;
                                                    play_menu_sound(
                                                        &mut producer,
                                                        MenuSound::Error,
                                                        actual_sample_rate,
                                                        audio_volume as f32 / 100.0,
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        overlay_message = Some("NO RECORDING DATA".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Error,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    paused = false;
                                }
                                11 => {
                                    // Load recording
                                    if let Some(base) = recordings_dir() {
                                        let path = base.join(format!("{}.nrec", current_rom_name));
                                        match InputRecording::load_from_file(
                                            path.to_str().unwrap_or(""),
                                        ) {
                                            Ok(loaded) => {
                                                let count = loaded.frame_count();
                                                recorder = loaded;
                                                overlay_message =
                                                    Some(format!("LOADED {} FRAMES", count));
                                                overlay_timer = 90;
                                                play_menu_sound(
                                                    &mut producer,
                                                    MenuSound::Confirm,
                                                    actual_sample_rate,
                                                    audio_volume as f32 / 100.0,
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("[recording] Load error: {}", e);
                                                overlay_message = Some("LOAD FAILED".to_string());
                                                overlay_timer = 90;
                                                play_menu_sound(
                                                    &mut producer,
                                                    MenuSound::Error,
                                                    actual_sample_rate,
                                                    audio_volume as f32 / 100.0,
                                                );
                                            }
                                        }
                                    }
                                    paused = false;
                                }
                                12 => {
                                    // Export FM2
                                    if recorder.frame_count() > 0 {
                                        if let Some(base) = recordings_dir() {
                                            let _ = std::fs::create_dir_all(&base);
                                            let path =
                                                base.join(format!("{}.fm2", current_rom_name));
                                            match recorder.export_fm2(
                                                path.to_str().unwrap_or("recording.fm2"),
                                                &current_rom_name,
                                            ) {
                                                Ok(()) => {
                                                    overlay_message =
                                                        Some("FM2 EXPORTED".to_string());
                                                    overlay_timer = 90;
                                                    play_menu_sound(
                                                        &mut producer,
                                                        MenuSound::Confirm,
                                                        actual_sample_rate,
                                                        audio_volume as f32 / 100.0,
                                                    );
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "[recording] FM2 export error: {}",
                                                        e
                                                    );
                                                    overlay_message =
                                                        Some("EXPORT FAILED".to_string());
                                                    overlay_timer = 90;
                                                    play_menu_sound(
                                                        &mut producer,
                                                        MenuSound::Error,
                                                        actual_sample_rate,
                                                        audio_volume as f32 / 100.0,
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        overlay_message = Some("NO RECORDING DATA".to_string());
                                        overlay_timer = 90;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Error,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    paused = false;
                                }
                                13 => {
                                    // Controls reference page
                                    controls_submenu = true;
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Confirm,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                }
                                _ => {}
                            }
                        }
                        if input.back {
                            paused = false; // ESC again resumes
                            play_menu_sound(
                                &mut producer,
                                MenuSound::Back,
                                actual_sample_rate,
                                audio_volume as f32 / 100.0,
                            );
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
                                        let _ = bus.service_dmc_dma(cpu.is_odd_cycle());

                                        if bus.ppu.frame_complete() {
                                            break;
                                        }
                                    }

                                    // End APU frame
                                    bus.apu.end_frame();

                                    if ff == frame_count - 1 {
                                        std::mem::swap(
                                            &mut audio_swap_buf,
                                            &mut bus.apu.sample_buffer,
                                        );
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
                                    if let Err(e) =
                                        script.on_frame(&ram_snapshot, frame_counter as u64)
                                    {
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
                                if let Some((_, gamepad)) =
                                    g.gamepads().find(|(_, gp)| gp.is_connected())
                                {
                                    l = gamepad.is_pressed(Button::LeftTrigger)
                                        || gamepad.is_pressed(Button::LeftTrigger2);
                                    r = gamepad.is_pressed(Button::RightTrigger)
                                        || gamepad.is_pressed(Button::RightTrigger2);
                                }
                            }
                            (false, false, l, r)
                        } else {
                            handle_input(
                                &window,
                                bus,
                                &mut gilrs,
                                frame_counter,
                                &config.input_bindings,
                                &mut stick_state_p1,
                                &mut stick_state_p2,
                            )
                        };

                        // Recording: capture current joypad state after input handling
                        if !quick_overlay && recorder.is_recording() {
                            let p1 = joypad_to_byte(bus, 1);
                            let p2 = joypad_to_byte(bus, 2);
                            recorder.record_frame(p1, p2);
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
                            if netplay.should_send_keepalive() {
                                netplay.send_keepalive();
                            }
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
                            let remote_bits = netplay
                                .receive_input()
                                .unwrap_or(netplay.last_remote_input());
                            let (ra, rb, rsel, rst, rup, rdn, rlt, rrt) =
                                NetplaySession::decode_input(remote_bits);

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
                                let (la, lb, lsel, lst, lup, ldn, llt, lrt) =
                                    NetplaySession::decode_input(local_bits);
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
                            if netplay.should_send_keepalive() {
                                netplay.send_keepalive();
                            }
                            let _ = netplay.receive_input();
                            if netplay.is_connected() {
                                overlay_message = Some(format!(
                                    "NETPLAY CONNECTED (P{})",
                                    netplay.local_player + 1
                                ));
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
                            pause_save_label =
                                format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                            pause_load_label =
                                format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                            overlay_message = Some("SLOT 1 SELECTED".to_string());
                            overlay_timer = 60;
                        }
                        if window.is_key_pressed(Key::F3, KeyRepeat::No) {
                            current_save_slot = 2;
                            pause_save_label =
                                format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                            pause_load_label =
                                format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                            overlay_message = Some("SLOT 2 SELECTED".to_string());
                            overlay_timer = 60;
                        }
                        if window.is_key_pressed(Key::F4, KeyRepeat::No) {
                            current_save_slot = 3;
                            pause_save_label =
                                format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                            pause_load_label =
                                format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                            overlay_message = Some("SLOT 3 SELECTED".to_string());
                            overlay_timer = 60;
                        }
                        if window.is_key_pressed(Key::F6, KeyRepeat::No) {
                            current_save_slot = 4;
                            pause_save_label =
                                format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                            pause_load_label =
                                format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                            overlay_message = Some("SLOT 4 SELECTED".to_string());
                            overlay_timer = 60;
                        }

                        // F5 quick save, F9 quick load (using current slot)
                        if window.is_key_pressed(Key::F5, KeyRepeat::No) {
                            if save_state(bus, cpu, &config, current_save_slot) {
                                overlay_message =
                                    Some(format!("STATE {} SAVED", current_save_slot));
                                overlay_timer = 90;
                            } else {
                                overlay_message = Some("SAVE FAILED".to_string());
                                overlay_timer = 90;
                            }
                        }
                        if window.is_key_pressed(Key::F9, KeyRepeat::No) {
                            if load_state(bus, cpu, &config, current_save_slot) {
                                overlay_message =
                                    Some(format!("STATE {} LOADED", current_save_slot));
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

                        // F10 perf overlay cycle: Off -> Basic -> Detailed -> Off
                        if window.is_key_pressed(Key::F10, KeyRepeat::No) {
                            let prev_level = overlay_level;
                            overlay_level = overlay_level.next();
                            if overlay_level == PerfOverlayLevel::Off {
                                fps_display.clear();
                                detail_display.clear();
                            }
                            if should_prime_detail_sampling(prev_level, overlay_level) {
                                detail_tick = 59;
                                detail_display.clear();
                            }
                            // Reset fps state when enabling from Off so the first
                            // displayed measurement window starts fresh, not with
                            // stale elapsed time from when the overlay was hidden.
                            if should_reset_fps_on_transition(prev_level, overlay_level) {
                                fps_frames = 0;
                                fps_timer = std::time::Instant::now();
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
                                )
                                .expect("Failed to create fullscreen window");
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
                                )
                                .expect("Failed to create window");
                            }
                            window.set_target_fps(0); // Disabled: custom hybrid pacer handles timing
                            overlay_message = Some(if is_fullscreen {
                                "FULLSCREEN".to_string()
                            } else {
                                "WINDOWED".to_string()
                            });
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
                                if audio_volume == 0 {
                                    audio_volume = 100;
                                }
                                let bars = (audio_volume / 10) as usize;
                                let bar: String = "#".repeat(bars) + &".".repeat(10 - bars);
                                overlay_message =
                                    Some(format!("VOLUME: [{}] {}%", bar, audio_volume));
                            }
                            overlay_timer = 60;
                        }

                        if window.is_key_pressed(Key::F1, KeyRepeat::No) {
                            crt_enabled = !crt_enabled;
                            config.crt_enabled = crt_enabled;
                            save_config(&config);
                            overlay_message = Some(if crt_enabled {
                                "CRT FILTER: ON".to_string()
                            } else {
                                "CRT FILTER: OFF".to_string()
                            });
                            overlay_timer = 90; // 1.5 seconds
                        }

                        // Brightness/Contrast OSD hotkeys
                        if window.is_key_pressed(Key::Minus, KeyRepeat::Yes) {
                            config.crt_config.brightness =
                                (config.crt_config.brightness as i16 - 5).clamp(-50, 50) as i8;
                            save_config(&config);
                            osd_type = OsdType::Brightness;
                            osd_value = config.crt_config.brightness as i32;
                            osd_timer = 120;
                        }
                        if window.is_key_pressed(Key::Equal, KeyRepeat::Yes) {
                            config.crt_config.brightness =
                                (config.crt_config.brightness as i16 + 5).clamp(-50, 50) as i8;
                            save_config(&config);
                            osd_type = OsdType::Brightness;
                            osd_value = config.crt_config.brightness as i32;
                            osd_timer = 120;
                        }
                        if window.is_key_pressed(Key::LeftBracket, KeyRepeat::Yes) {
                            config.crt_config.contrast =
                                (config.crt_config.contrast as i16 - 5).clamp(-50, 50) as i8;
                            save_config(&config);
                            osd_type = OsdType::Contrast;
                            osd_value = config.crt_config.contrast as i32;
                            osd_timer = 120;
                        }
                        if window.is_key_pressed(Key::RightBracket, KeyRepeat::Yes) {
                            config.crt_config.contrast =
                                (config.crt_config.contrast as i16 + 5).clamp(-50, 50) as i8;
                            save_config(&config);
                            osd_type = OsdType::Contrast;
                            osd_value = config.crt_config.contrast as i32;
                            osd_timer = 120;
                        }

                        // Shift+R toggle recording
                        if window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift)
                        {
                            if window.is_key_pressed(Key::R, KeyRepeat::No) {
                                if recorder.is_recording() {
                                    recorder.stop_recording();
                                    overlay_message = Some(format!(
                                        "REC STOPPED ({} FRAMES)",
                                        recorder.frame_count()
                                    ));
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
                                    overlay_message =
                                        Some(format!("PLAYING {} FRAMES", recorder.frame_count()));
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
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Confirm,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
                            }
                        } else {
                            if quick_overlay_lr_frames > 0 && quick_overlay_lr_frames < 20 {
                                // Short tap — ignore
                            }
                            quick_overlay_lr_frames = 0;
                        }

                        // Quick overlay input handling
                        if quick_overlay {
                            let input = poll_menu_input(
                                &window,
                                &mut gilrs,
                                &mut repeat_tracker,
                                config.input_bindings.controller_p1.deadzone,
                                &mut stick_state_menu,
                            );
                            let overlay_item_count: usize = 6;

                            if input.up && quick_overlay_selected > 0 {
                                quick_overlay_selected -= 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            if input.down && quick_overlay_selected < overlay_item_count - 1 {
                                quick_overlay_selected += 1;
                                if sound_cooldown == 0 {
                                    play_menu_sound(
                                        &mut producer,
                                        MenuSound::Cursor,
                                        actual_sample_rate,
                                        audio_volume as f32 / 100.0,
                                    );
                                    sound_cooldown = 3;
                                }
                            }
                            // L/R cycles save slot on save/load items
                            if quick_overlay_selected == 1 || quick_overlay_selected == 2 {
                                if input.left {
                                    current_save_slot = if current_save_slot == 1 {
                                        5
                                    } else {
                                        current_save_slot - 1
                                    };
                                    pause_save_label =
                                        format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                                    pause_load_label =
                                        format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                                }
                                if input.right {
                                    current_save_slot = if current_save_slot == 5 {
                                        1
                                    } else {
                                        current_save_slot + 1
                                    };
                                    pause_save_label =
                                        format!("SAVE STATE  (F5)  [SLOT {}]", current_save_slot);
                                    pause_load_label =
                                        format!("LOAD STATE  (F9)  [SLOT {}]", current_save_slot);
                                }
                            }
                            if input.confirm {
                                match quick_overlay_selected {
                                    0 => {
                                        // Resume
                                        quick_overlay = false;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    1 => {
                                        // Save State
                                        if save_state(bus, cpu, &config, current_save_slot) {
                                            thumbnail_cache[(current_save_slot as usize)
                                                .saturating_sub(1)
                                                .min(3)] =
                                                load_thumbnail(&config, current_save_slot);
                                            overlay_message =
                                                Some(format!("STATE {} SAVED", current_save_slot));
                                            overlay_timer = 90;
                                        } else {
                                            overlay_message = Some("SAVE FAILED".to_string());
                                            overlay_timer = 90;
                                        }
                                        quick_overlay = false;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    2 => {
                                        // Load State
                                        if load_state(bus, cpu, &config, current_save_slot) {
                                            overlay_message =
                                                Some(format!("STATE {} LOADED", current_save_slot));
                                            overlay_timer = 90;
                                        } else {
                                            overlay_message = Some("NO SAVE FOUND".to_string());
                                            overlay_timer = 90;
                                        }
                                        quick_overlay = false;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    3 => {
                                        // Toggle Favorite
                                        let added = toggle_favorite(&mut config, &current_rom_path);
                                        save_config(&config);
                                        favorites_valid = config
                                            .favorite_games
                                            .iter()
                                            .map(|p| std::path::Path::new(p.as_str()).exists())
                                            .collect();
                                        if added {
                                            overlay_message =
                                                Some("ADDED TO FAVORITES".to_string());
                                        } else {
                                            overlay_message =
                                                Some("REMOVED FROM FAVORITES".to_string());
                                        }
                                        overlay_timer = 90;
                                        quick_overlay = false;
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    4 => {
                                        // Full Pause Menu
                                        quick_overlay = false;
                                        paused = true;
                                        pause_selected = 0;
                                        pause_cursor_timer = 0;
                                        pause_cursor_visible = true;
                                        for slot in 0..4u8 {
                                            thumbnail_cache[slot as usize] =
                                                load_thumbnail(&config, slot + 1);
                                        }
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Confirm,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                    }
                                    5 => {
                                        // Return to Menu
                                        if let Some(ref bus) = game_bus {
                                            auto_save_sram(bus, &config);
                                        }
                                        game_bus = None;
                                        game_cpu = None;
                                        quick_overlay = false;
                                        repeat_tracker = RepeatTracker::new();
                                        emulator_state = EmulatorState::Menu(MenuState::new());
                                        window_title =
                                            format!("OxideNES v{}", env!("OXIDENES_VERSION"));
                                        window.set_title(&window_title);
                                        play_menu_sound(
                                            &mut producer,
                                            MenuSound::Back,
                                            actual_sample_rate,
                                            audio_volume as f32 / 100.0,
                                        );
                                        continue;
                                    }
                                    _ => {}
                                }
                            }
                            if input.back {
                                quick_overlay = false;
                                play_menu_sound(
                                    &mut producer,
                                    MenuSound::Back,
                                    actual_sample_rate,
                                    audio_volume as f32 / 100.0,
                                );
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

                    // Detailed perf overlay: sample render-stage timings once per ~60 frames (≈1 s).
                    // The Option<Instant> pattern has zero cost when do_detail_sample is false.
                    let do_detail_sample = overlay_level == PerfOverlayLevel::Detailed && {
                        detail_tick = detail_tick.wrapping_add(1);
                        detail_tick.is_multiple_of(60)
                    };
                    // Render game frame (skip when paused or overlay active — they have own rendering)
                    if !paused && !quick_overlay {
                        let dt = if barrel_distortion {
                            &distortion_table
                        } else {
                            &flat_distortion_table
                        };

                        // Stage 1: CRT filter (or passthrough scale)
                        let t_crt = if do_detail_sample {
                            Some(std::time::Instant::now())
                        } else {
                            None
                        };
                        if crt_enabled {
                            crt_filter(
                                &bus.ppu.frame_data,
                                &mut crt_buffer,
                                &sv_table,
                                dt,
                                &config.crt_config,
                                &mask_table,
                                config.crt_config.brightness as i32,
                                config.crt_config.contrast as i32,
                            );
                        } else {
                            scale_simple(&bus.ppu.frame_data, &mut crt_buffer);
                        }
                        if let Some(t) = t_crt {
                            perf_snapshot.crt_us = t.elapsed().as_micros() as u32;
                        }

                        // Stage 2: Post-process (bloom + glow + chromatic aberration)
                        let t_bloom = if do_detail_sample {
                            Some(std::time::Instant::now())
                        } else {
                            None
                        };
                        if crt_enabled {
                            // Phosphor bloom — bright pixels glow into neighbors
                            apply_phosphor_bloom(
                                &mut crt_buffer,
                                SCREEN_W,
                                SCREEN_H,
                                config.crt_config.phosphor_warmth as u32,
                            );
                            apply_scanline_glow(
                                &mut crt_buffer,
                                SCREEN_W,
                                SCREEN_H,
                                config.crt_config.phosphor_warmth as u32,
                            );
                            // Apply chromatic aberration to crt_buffer (screen area only)
                            if glass_intensity > 30 {
                                apply_chromatic_aberration(
                                    &mut ca_temp,
                                    &crt_buffer,
                                    &ca_table,
                                    SCREEN_W,
                                    SCREEN_H,
                                );
                                std::mem::swap(&mut crt_buffer, &mut ca_temp);
                            }
                        }
                        if let Some(t) = t_bloom {
                            perf_snapshot.bloom_us = t.elapsed().as_micros() as u32;
                        }

                        // Stage 3: Composite game output into TV frame
                        let t_composite = if do_detail_sample {
                            Some(std::time::Instant::now())
                        } else {
                            None
                        };
                        composite_screen_fast(
                            &mut composite_buffer,
                            &crt_buffer,
                            &screen_curve_table,
                            WINDOW_WIDTH,
                        );
                        if let Some(t) = t_composite {
                            perf_snapshot.composite_us = t.elapsed().as_micros() as u32;
                        }

                        // Stage 4: Glass / bezel effects
                        let t_glass = if do_detail_sample {
                            Some(std::time::Instant::now())
                        } else {
                            None
                        };
                        if crt_enabled && glass_intensity > 0 {
                            apply_glass_effects(
                                &mut composite_buffer,
                                &crt_buffer,
                                &glare_table,
                                &glass_thickness_table,
                                &ghost_alpha_table,
                                WINDOW_WIDTH,
                                glass_intensity,
                                false,
                                SCREEN_W,
                            );
                        }
                        if let Some(t) = t_glass {
                            perf_snapshot.glass_us = t.elapsed().as_micros() as u32;
                        }
                    } // end if !paused && !quick_overlay

                    // Latch detailed display string once per sample
                    if do_detail_sample {
                        fmt_detail_line(&mut detail_display, &perf_snapshot);
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
                                        composite_buffer[di] =
                                            (src_r << 16) | (composite_buffer[di] & 0x00FFFF);
                                    }
                                }
                                // Shift blue channel right (source = left neighbour)
                                for x in (sp..sw).rev() {
                                    let di = buf_y * WINDOW_WIDTH + sx + x;
                                    let si = di - sp;
                                    if di < composite_buffer.len() && si < composite_buffer.len() {
                                        let src_b = composite_buffer[si] & 0xFF;
                                        composite_buffer[di] =
                                            (composite_buffer[di] & 0xFFFF00) | src_b;
                                    }
                                }
                            }

                            // ── Per-pixel: desaturate, pump, snow, roll bar, speed lines ──
                            for x in 0..sw {
                                let idx = buf_y * WINDOW_WIDTH + sx + x;
                                if idx >= composite_buffer.len() {
                                    break;
                                }
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
                                let dist = if row >= roll_y {
                                    row - roll_y
                                } else {
                                    row + sh - roll_y
                                };
                                if dist < 40 {
                                    let fade = if dist < 10 {
                                        128u32
                                    } else if dist < 30 {
                                        160
                                    } else {
                                        192
                                    };
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

                                composite_buffer[idx] =
                                    (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
                            }
                        }

                        // ── 1. Tracking lines: 4 bright noise bands scrolling down ──
                        for band in 0..4u64 {
                            let by = ((fc.wrapping_mul(7).wrapping_add(band * 193)) % sh as u64)
                                as usize;
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

                    let has_rewind_bar =
                        !paused && is_rewinding && !rewind_buffer.snapshots.is_empty();

                    // Rewind buffer HUD bar (VCR style, top-right)
                    if has_rewind_bar {
                        let rewind_pct =
                            (rewind_buffer.snapshots.len() * 100) / rewind_buffer.max_snapshots;
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
                                    composite_buffer[idx] =
                                        if dx < filled { 0x00DDDD } else { 0x102030 };
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
                        draw_text(
                            &mut composite_buffer,
                            "<< REW",
                            osd_x,
                            osd_y,
                            0x00FFFF,
                            WINDOW_WIDTH,
                        );
                        // Fake backward-counting timecode  MM:SS:FF
                        let tc_total = 36000u64.saturating_sub(fc % 36000);
                        let display_s = tc_total / 60;
                        let mm = (display_s / 60) % 100;
                        let ss = display_s % 60;
                        let ff = tc_total % 60;
                        let tc_buf: [u8; 8] = [
                            b'0' + (mm / 10) as u8,
                            b'0' + (mm % 10) as u8,
                            b':',
                            b'0' + (ss / 10) as u8,
                            b'0' + (ss % 10) as u8,
                            b':',
                            b'0' + (ff / 10) as u8,
                            b'0' + (ff % 10) as u8,
                        ];
                        // SAFETY: all bytes are ASCII digits or ':', guaranteed valid UTF-8
                        let tc = unsafe { std::str::from_utf8_unchecked(&tc_buf) };
                        draw_text(
                            &mut composite_buffer,
                            tc,
                            osd_x + 8,
                            osd_y + 10,
                            0x00DDDD,
                            WINDOW_WIDTH,
                        );
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

                    // === CRT OSD Bar (brightness/contrast) ===
                    if osd_timer > 0 {
                        osd_timer -= 1;

                        // Fade: full brightness for first 90 frames, then fade over last 30
                        let alpha = if osd_timer > 30 {
                            255u32
                        } else {
                            osd_timer * 255 / 30
                        };

                        if osd_type != OsdType::None {
                            // CRT TV OSD: thin green phosphor pipes ||||||----- spanning full screen width
                            let pad: usize = 40;
                            let icon_space: usize = 30; // space for icon on the left
                            let val_space: usize = 40; // space for value on the right
                            let osd_left = SCREEN_X + pad + icon_space;
                            let osd_right = SCREEN_X + SCREEN_W - pad - val_space;
                            let available_w = osd_right - osd_left;

                            let num_bars: usize = 50;
                            let bar_w: usize = 2; // thin pipes
                            let bar_gap: usize =
                                ((available_w - num_bars * bar_w) / (num_bars - 1)).max(2);
                            let bar_stride = bar_w + bar_gap;
                            let total_w = num_bars * bar_stride - bar_gap;
                            let bar_h: usize = 28;
                            let dash_h: usize = 2;

                            let bar_y = SCREEN_Y + SCREEN_H - 52;
                            let bar_x = osd_left + (available_w - total_w) / 2;

                            // Green phosphor colors
                            let bright_g = (alpha * 0xFF) >> 8;
                            let bright_b = (alpha * 0x30) >> 8;

                            let dim_g = (alpha * 0x40) >> 8;
                            let dim_b = (alpha * 0x10) >> 8;

                            let icon_g = (alpha * 0xDD) >> 8;
                            let icon_b = (alpha * 0x20) >> 8;
                            let icon_color = (icon_g << 8) | icon_b;

                            // === Draw icon (pixel art) to the left of the bar ===
                            let icon_x = bar_x - icon_space;
                            let icon_y = bar_y + bar_h / 2 - 7; // center vertically with bar

                            // Helper closure to set a pixel with bounds checking
                            let set_px = |buf: &mut [u32], px: usize, py: usize, color: u32| {
                                if px < WINDOW_WIDTH && py < WINDOW_HEIGHT {
                                    let idx = py * WINDOW_WIDTH + px;
                                    if idx < buf.len() {
                                        buf[idx] = color;
                                    }
                                }
                            };

                            match osd_type {
                                OsdType::Brightness => {
                                    // Sun icon: 13x13 pixel art
                                    // Center dot (3x3)
                                    for dy in 0..3usize {
                                        for dx in 0..3usize {
                                            set_px(
                                                &mut composite_buffer,
                                                icon_x + 5 + dx,
                                                icon_y + 5 + dy,
                                                icon_color,
                                            );
                                        }
                                    }
                                    // Circle (radius ~4)
                                    let circle_pts: [(usize, usize); 16] = [
                                        (6, 2),
                                        (7, 2),
                                        (8, 3),
                                        (9, 4),
                                        (9, 5),
                                        (9, 6),
                                        (9, 7),
                                        (8, 8),
                                        (7, 9),
                                        (6, 9),
                                        (5, 8),
                                        (4, 9),
                                        (3, 8),
                                        (2, 7),
                                        (2, 6),
                                        (2, 5),
                                    ];
                                    let circle_pts2: [(usize, usize); 4] =
                                        [(2, 4), (3, 3), (4, 2), (5, 2)];
                                    for &(dx, dy) in circle_pts.iter().chain(circle_pts2.iter()) {
                                        set_px(
                                            &mut composite_buffer,
                                            icon_x + dx,
                                            icon_y + dy,
                                            icon_color,
                                        );
                                    }
                                    // Rays (8 directions, 2px each)
                                    let rays: [(i32, i32); 8] = [
                                        (0, -1),
                                        (0, 1),
                                        (-1, 0),
                                        (1, 0),
                                        (-1, -1),
                                        (1, -1),
                                        (-1, 1),
                                        (1, 1),
                                    ];
                                    for &(rdx, rdy) in &rays {
                                        for dist in 5..7i32 {
                                            let rx = (6i32 + rdx * dist) as usize;
                                            let ry = (6i32 + rdy * dist) as usize;
                                            set_px(
                                                &mut composite_buffer,
                                                icon_x + rx,
                                                icon_y + ry,
                                                icon_color,
                                            );
                                        }
                                    }
                                }
                                OsdType::Contrast => {
                                    // Half-circle icon: 13x13 pixel art
                                    // Draw circle outline
                                    let outline: [(usize, usize); 24] = [
                                        (5, 0),
                                        (6, 0),
                                        (7, 0),
                                        (8, 1),
                                        (9, 2),
                                        (10, 3),
                                        (10, 4),
                                        (10, 5),
                                        (10, 6),
                                        (10, 7),
                                        (10, 8),
                                        (9, 9),
                                        (8, 10),
                                        (7, 11),
                                        (6, 11),
                                        (5, 11),
                                        (4, 10),
                                        (3, 9),
                                        (2, 8),
                                        (2, 7),
                                        (2, 6),
                                        (2, 5),
                                        (2, 4),
                                        (3, 2),
                                    ];
                                    let outline2: [(usize, usize); 2] = [(4, 1), (3, 3)];
                                    for &(dx, dy) in outline.iter().chain(outline2.iter()) {
                                        set_px(
                                            &mut composite_buffer,
                                            icon_x + dx,
                                            icon_y + dy,
                                            icon_color,
                                        );
                                    }
                                    // Fill left half (x <= 6)
                                    for dy in 1..11usize {
                                        let x_start = match dy {
                                            1 => 4,
                                            2 => 3,
                                            3 => 3,
                                            9 => 3,
                                            10 => 4,
                                            _ => 2,
                                        };
                                        for dx in x_start..7usize {
                                            set_px(
                                                &mut composite_buffer,
                                                icon_x + dx,
                                                icon_y + dy,
                                                icon_color,
                                            );
                                        }
                                    }
                                }
                                OsdType::None => {}
                            }

                            // === Draw bars ===
                            let filled_count =
                                (((osd_value + 50) as usize) * num_bars / 100).min(num_bars);

                            for i in 0..num_bars {
                                let sx = bar_x + i * bar_stride;
                                if i < filled_count {
                                    // Filled: thin bright green vertical pipe |
                                    for y in bar_y..(bar_y + bar_h).min(WINDOW_HEIGHT) {
                                        for x in sx..(sx + bar_w).min(WINDOW_WIDTH) {
                                            let idx = y * WINDOW_WIDTH + x;
                                            if idx < composite_buffer.len() {
                                                let bg = composite_buffer[idx];
                                                let bg_r = (bg >> 16) & 0xFF;
                                                let bg_g = (bg >> 8) & 0xFF;
                                                let bg_b = bg & 0xFF;
                                                let inv = 255 - alpha;
                                                let r = (bg_r * inv) >> 8;
                                                let g = (bright_g * alpha + bg_g * inv) >> 8;
                                                let b = (bright_b * alpha + bg_b * inv) >> 8;
                                                composite_buffer[idx] = (r << 16) | (g << 8) | b;
                                            }
                                        }
                                    }
                                } else {
                                    // Empty: thin dim horizontal dash - at vertical center
                                    let dash_y = bar_y + bar_h / 2 - dash_h / 2;
                                    for y in dash_y..(dash_y + dash_h).min(WINDOW_HEIGHT) {
                                        for x in sx..(sx + bar_w).min(WINDOW_WIDTH) {
                                            let idx = y * WINDOW_WIDTH + x;
                                            if idx < composite_buffer.len() {
                                                let bg = composite_buffer[idx];
                                                let bg_r = (bg >> 16) & 0xFF;
                                                let bg_g = (bg >> 8) & 0xFF;
                                                let bg_b = bg & 0xFF;
                                                let inv = 255 - alpha;
                                                let r = (bg_r * inv) >> 8;
                                                let g = (dim_g * alpha + bg_g * inv) >> 8;
                                                let b = (dim_b * alpha + bg_b * inv) >> 8;
                                                composite_buffer[idx] = (r << 16) | (g << 8) | b;
                                            }
                                        }
                                    }
                                }
                            }

                            // Center tick at zero point
                            let center_seg = num_bars / 2;
                            let center_x = bar_x + center_seg * bar_stride;
                            for dy in 0..5usize {
                                let ty = bar_y.saturating_sub(5) + dy;
                                if ty < WINDOW_HEIGHT && center_x < WINDOW_WIDTH {
                                    let idx = ty * WINDOW_WIDTH + center_x;
                                    if idx < composite_buffer.len() {
                                        composite_buffer[idx] = icon_color;
                                    }
                                    if center_x + 1 < WINDOW_WIDTH {
                                        let idx2 = ty * WINDOW_WIDTH + center_x + 1;
                                        if idx2 < composite_buffer.len() {
                                            composite_buffer[idx2] = icon_color;
                                        }
                                    }
                                }
                            }

                            // Value text to the right
                            let val_text = format!("{:+}", osd_value);
                            let val_x = bar_x + total_w + 10;
                            draw_text(
                                &mut composite_buffer,
                                &val_text,
                                val_x,
                                bar_y + 10,
                                icon_color,
                                WINDOW_WIDTH,
                            );

                            // CRT scanline effect over OSD region
                            let osd_region_x0 = (SCREEN_X + pad).saturating_sub(4);
                            let osd_region_x1 = (SCREEN_X + SCREEN_W - pad + 4).min(WINDOW_WIDTH);
                            let osd_region_y0 = bar_y.saturating_sub(8);
                            let osd_region_y1 = (bar_y + bar_h + 4).min(WINDOW_HEIGHT);
                            for y in osd_region_y0..osd_region_y1 {
                                if y % 2 == 1 {
                                    for x in osd_region_x0..osd_region_x1 {
                                        let idx = y * WINDOW_WIDTH + x;
                                        if idx < composite_buffer.len() {
                                            let p = composite_buffer[idx];
                                            let r = (((p >> 16) & 0xFF) * 154) >> 8;
                                            let g = (((p >> 8) & 0xFF) * 154) >> 8;
                                            let b = ((p & 0xFF) * 154) >> 8;
                                            composite_buffer[idx] = (r << 16) | (g << 8) | b;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Achievement notification toasts (top-right, gold background)
                    {
                        let mut notify_y = SCREEN_Y + 40;
                        for notif in achievement_engine.notifications.iter() {
                            if notif.frames_remaining == 0 {
                                continue;
                            }
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
                            draw_text(
                                &mut composite_buffer,
                                text,
                                nx,
                                notify_y,
                                0xF8D878,
                                WINDOW_WIDTH,
                            );
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
                            draw_text(
                                &mut composite_buffer,
                                rec_text,
                                rx,
                                ry,
                                0xFF4444,
                                WINDOW_WIDTH,
                            );
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
                            draw_text(
                                &mut composite_buffer,
                                play_text,
                                px,
                                py,
                                0x44FF44,
                                WINDOW_WIDTH,
                            );
                        }
                    }

                    // Performance overlay (F10 cycles Off -> Basic -> Detailed -> Off)
                    if overlay_level != PerfOverlayLevel::Off {
                        let has_transport_hud =
                            !paused && (recorder.is_recording() || recorder.is_playing());
                        fps_frames += 1;
                        let elapsed = fps_timer.elapsed().as_secs_f64();
                        if elapsed >= 1.0 {
                            let avg_ms = (elapsed * 1000.0 / fps_frames as f64) as f32;
                            fmt_basic_line(&mut fps_display, fps_frames, avg_ms);
                            fps_frames = 0;
                            fps_timer = std::time::Instant::now();
                        }
                        if !fps_display.is_empty() {
                            let fx = SCREEN_X + SCREEN_W - fps_display.len() * 4 - 8;
                            let fy = perf_basic_overlay_y(has_rewind_bar);
                            draw_text(
                                &mut composite_buffer,
                                &fps_display,
                                fx,
                                fy,
                                0x00FF00,
                                WINDOW_WIDTH,
                            );
                        }
                        // Detailed: show sampled stage breakdown below the FPS line
                        if overlay_level == PerfOverlayLevel::Detailed && !detail_display.is_empty()
                        {
                            let dx = SCREEN_X + 8;
                            let dy = perf_detail_overlay_y(has_transport_hud);
                            draw_text(
                                &mut composite_buffer,
                                &detail_display,
                                dx,
                                dy,
                                0xFFAA00,
                                WINDOW_WIDTH,
                            );
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
                        draw_text(
                            &mut composite_buffer,
                            &cached_net_text,
                            nx,
                            ny,
                            0x44CCFF,
                            WINDOW_WIDTH,
                        );
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

                        draw_text(
                            &mut composite_buffer,
                            "=== CONTROLS ===",
                            help_x,
                            help_y,
                            0xFFFF00,
                            WINDOW_WIDTH,
                        );
                        help_y += 20;
                        draw_text(
                            &mut composite_buffer,
                            "WASD/ARROWS  D-PAD",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "K  A BUTTON",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "J  B BUTTON",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "ENTER  START",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "RSHIFT  SELECT",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "Z/X  TURBO A/B",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 24;
                        draw_text(
                            &mut composite_buffer,
                            "=== EMULATOR ===",
                            help_x,
                            help_y,
                            0xFFFF00,
                            WINDOW_WIDTH,
                        );
                        help_y += 20;
                        draw_text(
                            &mut composite_buffer,
                            "ESC  PAUSE MENU",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "TAB  FAST FORWARD",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "BACKSPACE  REWIND",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "M  MUTE/UNMUTE",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "F1  CRT FILTER",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "F2-F4,F6  SAVE SLOT",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "F5/F9  SAVE/LOAD",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "F7  RESET GAME",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "F8  SCREENSHOT",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "F10 PERF OVERLAY",
                            help_x,
                            help_y,
                            color,
                            WINDOW_WIDTH,
                        );
                        help_y += 12;
                        draw_text(
                            &mut composite_buffer,
                            "F12  THIS HELP",
                            help_x,
                            help_y,
                            dim,
                            WINDOW_WIDTH,
                        );
                        help_y += 24;
                        draw_text(
                            &mut composite_buffer,
                            "PRESS F12 TO CLOSE",
                            help_x,
                            help_y,
                            dim,
                            WINDOW_WIDTH,
                        );
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
                        draw_text(
                            &mut composite_buffer,
                            "RETURNING TO MENU",
                            text_x,
                            text_y,
                            0xFFFFFF,
                            WINDOW_WIDTH,
                        );
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
                            let title: String =
                                current_rom_name.to_uppercase().chars().take(24).collect();
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                &format!("\x11 {} \x11", title),
                                box_top + 1,
                                MENU_GOLD,
                            );
                        } else {
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "\x11 PAUSED \x11",
                                box_top + 1,
                                MENU_GOLD,
                            );
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
                        let script_status = if script_engine.as_ref().is_some_and(|s| s.active) {
                            "RELOAD SCRIPT"
                        } else {
                            "LOAD SCRIPT"
                        };
                        let ach_label = if achievement_engine.achievements.is_empty() {
                            "ACHIEVEMENTS".to_string()
                        } else {
                            format!(
                                "ACHIEVEMENTS ({}/{})",
                                achievement_engine.unlocked_count,
                                achievement_engine.achievements.len()
                            )
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
                        let items: Vec<&str> = vec![
                            "RESUME GAME",
                            &pause_save_label,
                            &pause_load_label,
                            &cheat_label,
                            &net_status,
                            script_status,
                            "UNLOAD SCRIPT",
                            &fav_label,
                            "RETURN TO MENU",
                            &ach_label,
                            &rec_label,
                            "LOAD RECORDING",
                            "EXPORT FM2",
                            "CONTROLS",
                        ];
                        for (i, item) in items.iter().enumerate() {
                            let row = box_top + 3 + i;
                            let is_selected = i == pause_selected;

                            if is_selected {
                                // Highlight bar (always visible)
                                draw_highlight_bar(
                                    &mut menu_framebuffer,
                                    row * 8,
                                    8,
                                    box_left * 8 + 4,
                                    box_right * 8 - 4,
                                    0x3C3C8C,
                                );
                                // Arrow blinks
                                if pause_cursor_visible {
                                    draw_char_8x8(
                                        &mut menu_framebuffer,
                                        '\x10',
                                        box_left + 1,
                                        row,
                                        MENU_WHITE,
                                    );
                                }
                            }

                            let color = if is_selected { MENU_WHITE } else { MENU_GRAY };
                            draw_text_8x8(&mut menu_framebuffer, item, box_left + 2, row, color);
                        }

                        // Hint at bottom of box
                        if pause_selected == 1 || pause_selected == 2 {
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "A:SELECT  L/R:SLOT",
                                box_bottom - 1,
                                MENU_DARK_GRAY,
                            );
                        } else {
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "ESC:RESUME  A:SELECT",
                                box_bottom - 1,
                                MENU_DARK_GRAY,
                            );
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
                                    if yt < 240 && x < 256 {
                                        menu_framebuffer[yt * 256 + x] = MENU_LIGHT_BLUE;
                                    }
                                    if yb < 240 && x < 256 {
                                        menu_framebuffer[yb * 256 + x] = MENU_LIGHT_BLUE;
                                    }
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
                                        if xr < 256 {
                                            menu_framebuffer[y * 256 + xr] = MENU_LIGHT_BLUE;
                                        }
                                    }
                                }
                            }

                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "\x11 CHEATS \x11",
                                cb_top + 1,
                                MENU_GOLD,
                            );

                            // List cheats + ADD CODE + CLEAR ALL
                            let max_visible = 12usize; // max rows for cheat list
                            let total_items = bus.cheats.len() + 2;
                            let scroll_offset = if cheats_selected >= max_visible {
                                cheats_selected - max_visible + 1
                            } else {
                                0
                            };

                            for i in 0..max_visible.min(total_items) {
                                let item_idx = scroll_offset + i;
                                if item_idx >= total_items {
                                    break;
                                }
                                let row = cb_top + 3 + i;
                                if row >= cb_bottom - 1 {
                                    break;
                                }
                                let is_sel = item_idx == cheats_selected;

                                if is_sel {
                                    draw_highlight_bar(
                                        &mut menu_framebuffer,
                                        row * 8,
                                        8,
                                        cb_left * 8 + 4,
                                        cb_right * 8 - 4,
                                        0x3C3C8C,
                                    );
                                    draw_char_8x8(
                                        &mut menu_framebuffer,
                                        '\x10',
                                        cb_left + 1,
                                        row,
                                        MENU_WHITE,
                                    );
                                }

                                let color = if is_sel { MENU_WHITE } else { MENU_GRAY };
                                if item_idx < bus.cheats.len() {
                                    let status = if bus.cheats[item_idx].enabled {
                                        "ON "
                                    } else {
                                        "OFF"
                                    };
                                    let label =
                                        format!("[{}] {}", status, bus.cheats[item_idx].code_str);
                                    draw_text_8x8(
                                        &mut menu_framebuffer,
                                        &label,
                                        cb_left + 2,
                                        row,
                                        color,
                                    );
                                } else if item_idx == bus.cheats.len() {
                                    draw_text_8x8(
                                        &mut menu_framebuffer,
                                        "ADD CODE...",
                                        cb_left + 2,
                                        row,
                                        color,
                                    );
                                } else {
                                    draw_text_8x8(
                                        &mut menu_framebuffer,
                                        "CLEAR ALL",
                                        cb_left + 2,
                                        row,
                                        color,
                                    );
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
                                draw_text_8x8(
                                    &mut menu_framebuffer,
                                    "GAME GENIE CODE:",
                                    7,
                                    input_y,
                                    0xF8D878,
                                );
                                let display = if cheat_input_buffer.is_empty() {
                                    "________"
                                } else {
                                    &cheat_input_buffer
                                };
                                draw_text_8x8(
                                    &mut menu_framebuffer,
                                    display,
                                    9,
                                    input_y + 1,
                                    0xFCFCFC,
                                );
                            }

                            // Status message
                            if cheat_message_timer > 0 {
                                cheat_message_timer -= 1;
                                if let Some(ref msg) = cheat_message {
                                    draw_text_centered_8x8(
                                        &mut menu_framebuffer,
                                        msg,
                                        cb_bottom - 2,
                                        0x44FF44,
                                    );
                                }
                                if cheat_message_timer == 0 {
                                    cheat_message = None;
                                }
                            }

                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "ESC:BACK  A:TOGGLE  BS:DEL",
                                cb_bottom - 1,
                                MENU_DARK_GRAY,
                            );
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
                                    if yt < 240 && x < 256 {
                                        menu_framebuffer[yt * 256 + x] = MENU_LIGHT_BLUE;
                                    }
                                    if yb < 240 && x < 256 {
                                        menu_framebuffer[yb * 256 + x] = MENU_LIGHT_BLUE;
                                    }
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
                                        if xr < 256 {
                                            menu_framebuffer[y * 256 + xr] = MENU_LIGHT_BLUE;
                                        }
                                    }
                                }
                            }

                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "\x11 NETPLAY \x11",
                                nb_top + 1,
                                MENU_GOLD,
                            );

                            // Status line
                            let status_color = if netplay.is_connected() {
                                0x44FF44u32
                            } else {
                                MENU_GRAY
                            };
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                netplay.status_text(),
                                nb_top + 2,
                                status_color,
                            );

                            let host_label = format!("HOST (PORT {})", netplay.port);
                            let delay_str = format!("INPUT DELAY: {}", netplay.input_delay);
                            let port_str = if netplay_editing_port {
                                format!("PORT: {}_", netplay_port_input)
                            } else {
                                format!("PORT: {}", netplay_port_input)
                            };
                            let np_items: [&str; 5] =
                                [&port_str, &host_label, "JOIN...", "DISCONNECT", &delay_str];
                            for (i, item) in np_items.iter().enumerate() {
                                let row = nb_top + 4 + i * 2;
                                let is_sel = i == netplay_selected;
                                if is_sel {
                                    draw_highlight_bar(
                                        &mut menu_framebuffer,
                                        row * 8,
                                        8,
                                        nb_left * 8 + 4,
                                        nb_right * 8 - 4,
                                        0x3C3C8C,
                                    );
                                    draw_char_8x8(
                                        &mut menu_framebuffer,
                                        '\x10',
                                        nb_left + 1,
                                        row,
                                        MENU_WHITE,
                                    );
                                }
                                let color = if is_sel { MENU_WHITE } else { MENU_GRAY };
                                draw_text_8x8(&mut menu_framebuffer, item, nb_left + 2, row, color);
                            }

                            // IP input overlay when editing
                            if netplay_ip_editing {
                                let ip_y = (nb_top + 9) * 8;
                                for dy in 0..16 {
                                    for dx in 0..128 {
                                        let x = 64 + dx;
                                        let y = ip_y + dy;
                                        if y < 240 && x < 256 {
                                            menu_framebuffer[y * 256 + x] = 0x000030;
                                        }
                                    }
                                }
                                draw_text_8x8(
                                    &mut menu_framebuffer,
                                    "IP:PORT:",
                                    9,
                                    nb_top + 9,
                                    0xF8D878,
                                );
                                let ip_display = if netplay_ip_input.is_empty() {
                                    "_"
                                } else {
                                    &netplay_ip_input
                                };
                                draw_text_8x8(
                                    &mut menu_framebuffer,
                                    ip_display,
                                    9,
                                    nb_top + 11,
                                    0xFCFCFC,
                                );
                            }

                            // Port input overlay when editing
                            if netplay_editing_port {
                                let port_y = (nb_top + 5) * 8;
                                for dy in 0..16 {
                                    for dx in 0..80 {
                                        let x = 88 + dx;
                                        let y = port_y + dy;
                                        if y < 240 && x < 256 {
                                            menu_framebuffer[y * 256 + x] = 0x000030;
                                        }
                                    }
                                }
                                draw_text_8x8(
                                    &mut menu_framebuffer,
                                    "PORT:",
                                    12,
                                    nb_top + 5,
                                    0xF8D878,
                                );
                                let port_display = if netplay_port_input.is_empty() {
                                    "_"
                                } else {
                                    &netplay_port_input
                                };
                                draw_text_8x8(
                                    &mut menu_framebuffer,
                                    port_display,
                                    18,
                                    nb_top + 5,
                                    0xFCFCFC,
                                );
                            }

                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "ESC:BACK  A:SELECT",
                                nb_bottom - 1,
                                MENU_DARK_GRAY,
                            );
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
                                    if yt < 240 && x < 256 {
                                        menu_framebuffer[yt * 256 + x] = MENU_GOLD;
                                    }
                                    if yb < 240 && x < 256 {
                                        menu_framebuffer[yb * 256 + x] = MENU_GOLD;
                                    }
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
                                        if xr < 256 {
                                            menu_framebuffer[y * 256 + xr] = MENU_GOLD;
                                        }
                                    }
                                }
                            }

                            let title = if achievement_engine.game_title.is_empty() {
                                "ACHIEVEMENTS".to_string()
                            } else {
                                achievement_engine.game_title.to_string()
                            };
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                &title,
                                ab_top + 1,
                                MENU_GOLD,
                            );

                            let stats = format!(
                                "{}/{} UNLOCKED  {}PTS",
                                achievement_engine.unlocked_count,
                                achievement_engine.achievements.len(),
                                achievement_engine.total_points
                            );
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                &stats,
                                ab_top + 2,
                                MENU_WHITE,
                            );

                            // List achievements (up to 16 visible)
                            let max_visible =
                                (ab_bottom - ab_top - 4).min(achievement_engine.achievements.len());
                            for (i, ach) in achievement_engine
                                .achievements
                                .iter()
                                .take(max_visible)
                                .enumerate()
                            {
                                let row = ab_top + 4 + i;
                                if row >= ab_bottom - 1 {
                                    break;
                                }
                                let icon = if ach.unlocked { "\x0F" } else { "." };
                                let label = format!("{} {} ({})", icon, ach.title, ach.points);
                                let color = if ach.unlocked { 0x44FF44u32 } else { MENU_GRAY };
                                draw_text_8x8(
                                    &mut menu_framebuffer,
                                    &label,
                                    ab_left + 1,
                                    row,
                                    color,
                                );
                            }

                            if achievement_engine.achievements.is_empty() {
                                draw_text_centered_8x8(
                                    &mut menu_framebuffer,
                                    "NO ACHIEVEMENTS LOADED",
                                    ab_top + 6,
                                    MENU_DARK_GRAY,
                                );
                                draw_text_centered_8x8(
                                    &mut menu_framebuffer,
                                    "PLACE JSON FILES IN",
                                    ab_top + 8,
                                    MENU_DARK_GRAY,
                                );
                                draw_text_centered_8x8(
                                    &mut menu_framebuffer,
                                    "~/.nes-emulator/",
                                    ab_top + 10,
                                    MENU_DARK_GRAY,
                                );
                                draw_text_centered_8x8(
                                    &mut menu_framebuffer,
                                    "achievements/",
                                    ab_top + 11,
                                    MENU_DARK_GRAY,
                                );
                            }

                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "ESC:BACK",
                                ab_bottom - 1,
                                MENU_DARK_GRAY,
                            );
                        }

                        // Controls reference page overlay
                        if controls_submenu {
                            // Fill entire framebuffer for full-screen reference page
                            for pixel in menu_framebuffer.iter_mut().take(256 * 240) {
                                *pixel = MENU_BG;
                            }

                            // Title
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "\x11 CONTROLS \x11",
                                1,
                                MENU_GOLD,
                            );

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
                                ("Up", &kb1.up, &kb2.up),
                                ("Down", &kb1.down, &kb2.down),
                                ("Left", &kb1.left, &kb2.left),
                                ("Right", &kb1.right, &kb2.right),
                                ("A", &kb1.a, &kb2.a),
                                ("B", &kb1.b, &kb2.b),
                                ("Start", &kb1.start, &kb2.start),
                                ("Select", &kb1.select, &kb2.select),
                                ("TrboA", &kb1.turbo_a, &kb2.turbo_a),
                                ("TrboB", &kb1.turbo_b, &kb2.turbo_b),
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
                            draw_text_8x8(
                                &mut menu_framebuffer,
                                "CONTROLLER P2",
                                18,
                                15,
                                MENU_GOLD,
                            );

                            let ct1 = &config.input_bindings.controller_p1;
                            let ct2 = &config.input_bindings.controller_p2;
                            let ct_rows: [(&str, &str, &str); 6] = [
                                ("A", &ct1.a, &ct2.a),
                                ("B", &ct1.b, &ct2.b),
                                ("TrboA", &ct1.turbo_a, &ct2.turbo_a),
                                ("TrboB", &ct1.turbo_b, &ct2.turbo_b),
                                ("Start", &ct1.start, &ct2.start),
                                ("Select", &ct1.select, &ct2.select),
                            ];
                            for (i, (label, v1, v2)) in ct_rows.iter().enumerate() {
                                let y = 16 + i;
                                draw_text_8x8(&mut menu_framebuffer, label, 2, y, MENU_GRAY);
                                draw_text_8x8(&mut menu_framebuffer, v1, 9, y, MENU_WHITE);
                                draw_text_8x8(&mut menu_framebuffer, label, 18, y, MENU_GRAY);
                                draw_text_8x8(&mut menu_framebuffer, v2, 25, y, MENU_WHITE);
                            }

                            // --- System Shortcuts ---
                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "SYSTEM SHORTCUTS",
                                23,
                                MENU_GOLD,
                            );

                            draw_text_8x8(&mut menu_framebuffer, "Pause", 2, 24, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Escape", 9, 24, MENU_WHITE);
                            draw_text_8x8(&mut menu_framebuffer, "Save", 18, 24, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "F5", 25, 24, MENU_WHITE);

                            draw_text_8x8(&mut menu_framebuffer, "Load", 2, 25, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "F9", 9, 25, MENU_WHITE);
                            draw_text_8x8(&mut menu_framebuffer, "Rewind", 18, 25, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Bksp", 25, 25, MENU_WHITE);

                            draw_text_8x8(&mut menu_framebuffer, "FF", 2, 26, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Tab", 9, 26, MENU_WHITE);
                            draw_text_8x8(&mut menu_framebuffer, "Reset", 18, 26, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Ctrl+R", 25, 26, MENU_WHITE);

                            draw_text_8x8(&mut menu_framebuffer, "Record", 2, 27, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Shft+R", 9, 27, MENU_WHITE);
                            draw_text_8x8(&mut menu_framebuffer, "Play", 18, 27, MENU_GRAY);
                            draw_text_8x8(&mut menu_framebuffer, "Shft+P", 25, 27, MENU_WHITE);

                            draw_text_centered_8x8(
                                &mut menu_framebuffer,
                                "ESC TO GO BACK",
                                29,
                                MENU_DARK_GRAY,
                            );
                        }

                        // Now pass through CRT filter (same as menu rendering)
                        let dt = if barrel_distortion {
                            &distortion_table
                        } else {
                            &flat_distortion_table
                        };
                        if crt_enabled {
                            crt_filter(
                                &menu_framebuffer,
                                &mut crt_buffer,
                                &sv_table,
                                dt,
                                &config.crt_config,
                                &mask_table,
                                config.crt_config.brightness as i32,
                                config.crt_config.contrast as i32,
                            );
                            // Phosphor bloom — bright pixels glow into neighbors
                            apply_phosphor_bloom(
                                &mut crt_buffer,
                                SCREEN_W,
                                SCREEN_H,
                                config.crt_config.phosphor_warmth as u32,
                            );
                            apply_scanline_glow(
                                &mut crt_buffer,
                                SCREEN_W,
                                SCREEN_H,
                                config.crt_config.phosphor_warmth as u32,
                            );
                            // Apply chromatic aberration to crt_buffer (screen area only)
                            if glass_intensity > 30 {
                                apply_chromatic_aberration(
                                    &mut ca_temp,
                                    &crt_buffer,
                                    &ca_table,
                                    SCREEN_W,
                                    SCREEN_H,
                                );
                                std::mem::swap(&mut crt_buffer, &mut ca_temp);
                            }
                        } else {
                            scale_simple(&menu_framebuffer, &mut crt_buffer);
                        }
                        composite_screen_fast(
                            &mut composite_buffer,
                            &crt_buffer,
                            &screen_curve_table,
                            WINDOW_WIDTH,
                        );
                        if crt_enabled && glass_intensity > 0 {
                            apply_glass_effects(
                                &mut composite_buffer,
                                &crt_buffer,
                                &glare_table,
                                &glass_thickness_table,
                                &ghost_alpha_table,
                                WINDOW_WIDTH,
                                glass_intensity,
                                false,
                                SCREEN_W,
                            );
                        }

                        // Render save state thumbnail in pause menu(composite buffer)
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
                                    if src + 2 >= thumb_data.len() {
                                        continue;
                                    }
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
                            draw_text(
                                &mut composite_buffer,
                                &label,
                                thumb_cx,
                                thumb_cy.saturating_sub(10),
                                0xF8D878,
                                WINDOW_WIDTH,
                            );
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
                            draw_text(
                                &mut composite_buffer,
                                &label,
                                thumb_cx,
                                thumb_cy.saturating_sub(10),
                                0xF8D878,
                                WINDOW_WIDTH,
                            );
                            let empty_x = thumb_cx + (thumb_w - 5 * 4) / 2;
                            let empty_y = thumb_cy + (thumb_h - 5) / 2;
                            draw_text(
                                &mut composite_buffer,
                                "EMPTY",
                                empty_x,
                                empty_y,
                                0x666688,
                                WINDOW_WIDTH,
                            );
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

                        // Draw centered overlay box (compact: 24 tiles wide × 14 tiles tall)
                        let box_left: usize = 4;
                        let box_right: usize = 28;
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
                                    if yt + 1 < 240 {
                                        menu_framebuffer[(yt + 1) * 256 + x] = 0x4080C0;
                                    }
                                }
                                if yb < 240 && x < 256 {
                                    menu_framebuffer[yb * 256 + x] = 0x4080C0;
                                    if yb > 0 {
                                        menu_framebuffer[(yb - 1) * 256 + x] = 0x4080C0;
                                    }
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
                                    if xl < 256 {
                                        menu_framebuffer[y * 256 + xl] = 0x4080C0;
                                        menu_framebuffer[y * 256 + xl + 1] = 0x4080C0;
                                    }
                                    if xr < 256 {
                                        menu_framebuffer[y * 256 + xr] = 0x4080C0;
                                        if xr > 0 {
                                            menu_framebuffer[y * 256 + xr - 1] = 0x4080C0;
                                        }
                                    }
                                }
                            }
                        }

                        // Title
                        draw_text_centered_8x8(
                            &mut menu_framebuffer,
                            "\x11 QUICK MENU \x11",
                            box_top + 1,
                            MENU_GOLD,
                        );

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
                                draw_highlight_bar(
                                    &mut menu_framebuffer,
                                    row * 8,
                                    8,
                                    box_left * 8 + 4,
                                    box_right * 8 - 4,
                                    0x2A2A6A,
                                );
                            }

                            let color = if is_selected { 0xFFFFFF } else { 0xA0A0A0 };
                            if is_selected {
                                draw_char_8x8(
                                    &mut menu_framebuffer,
                                    '\x10',
                                    box_left + 1,
                                    row,
                                    0xFFFFFF,
                                );
                            }
                            draw_text_8x8(&mut menu_framebuffer, item, box_left + 2, row, color);
                        }

                        // Hint at bottom
                        draw_text_centered_8x8(
                            &mut menu_framebuffer,
                            "B:CLOSE  L/R:SLOT",
                            box_bottom - 1,
                            0x606060,
                        );

                        // Pass through CRT filter (same as pause menu rendering)
                        let dt = if barrel_distortion {
                            &distortion_table
                        } else {
                            &flat_distortion_table
                        };
                        if crt_enabled {
                            crt_filter(
                                &menu_framebuffer,
                                &mut crt_buffer,
                                &sv_table,
                                dt,
                                &config.crt_config,
                                &mask_table,
                                config.crt_config.brightness as i32,
                                config.crt_config.contrast as i32,
                            );
                            apply_phosphor_bloom(
                                &mut crt_buffer,
                                SCREEN_W,
                                SCREEN_H,
                                config.crt_config.phosphor_warmth as u32,
                            );
                            apply_scanline_glow(
                                &mut crt_buffer,
                                SCREEN_W,
                                SCREEN_H,
                                config.crt_config.phosphor_warmth as u32,
                            );
                            if glass_intensity > 30 {
                                apply_chromatic_aberration(
                                    &mut ca_temp,
                                    &crt_buffer,
                                    &ca_table,
                                    SCREEN_W,
                                    SCREEN_H,
                                );
                                std::mem::swap(&mut crt_buffer, &mut ca_temp);
                            }
                        } else {
                            scale_simple(&menu_framebuffer, &mut crt_buffer);
                        }
                        composite_screen_fast(
                            &mut composite_buffer,
                            &crt_buffer,
                            &screen_curve_table,
                            WINDOW_WIDTH,
                        );
                        if crt_enabled && glass_intensity > 0 {
                            apply_glass_effects(
                                &mut composite_buffer,
                                &crt_buffer,
                                &glare_table,
                                &glass_thickness_table,
                                &ghost_alpha_table,
                                WINDOW_WIDTH,
                                glass_intensity,
                                false,
                                SCREEN_W,
                            );
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

        // === Hybrid frame pacer: sleep coarse + spin-wait precise ===
        {
            let elapsed = frame_start.elapsed();
            let target = std::time::Duration::from_nanos(frame_duration_ns);
            if elapsed < target {
                let remaining = target - elapsed;
                // Sleep for most of the remaining time (leave 2ms for spin-wait)
                if remaining > std::time::Duration::from_millis(2) {
                    std::thread::sleep(remaining - std::time::Duration::from_millis(2));
                }
                // Spin-wait for precise timing
                while frame_start.elapsed() < target {
                    std::hint::spin_loop();
                }
            }
            let now = std::time::Instant::now();
            let next = frame_start + std::time::Duration::from_nanos(frame_duration_ns);
            frame_start = match now.checked_duration_since(next) {
                // More than 3 frames behind: reset to prevent burst catch-up
                Some(behind) if behind > std::time::Duration::from_nanos(frame_duration_ns * 3) => {
                    now
                }
                // Normal case: advance to ideal next frame time (additive timing)
                _ => next,
            };
        }
    }
}

fn run_rom_import_command(args: &[String]) -> ! {
    let Some(source_dir) = option_value(args, "--import-roms") else {
        eprintln!("--import-roms requires a source directory");
        std::process::exit(2);
    };
    let mode = if args.iter().any(|arg| arg == "--import-mode") {
        let Some(value) = option_value(args, "--import-mode") else {
            eprintln!("--import-mode requires copy or symlink");
            std::process::exit(2);
        };
        match RomImportMode::parse(value) {
            Some(mode) => mode,
            None => {
                eprintln!("unsupported --import-mode: {value} (expected copy or symlink)");
                std::process::exit(2);
            }
        }
    } else {
        RomImportMode::Copy
    };

    match import_rom_folder(source_dir, mode) {
        Ok(summary) => {
            let mut config = load_config();
            let default_dir = point_config_at_default_library(&mut config);
            save_config(&config);
            println!(
                "Imported {} NES ROM(s) with {} mode into {}",
                summary.imported,
                summary.mode.as_str(),
                default_dir.display()
            );
            if summary.skipped_existing > 0 {
                println!(
                    "Skipped {} existing target file(s)",
                    summary.skipped_existing
                );
            }
            if summary.skipped_entries > 0 {
                println!("Skipped {} non-NES entry(s)", summary.skipped_entries);
            }
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!(
                "ROM import failed for target {}: {}",
                default_rom_library_dir().display(),
                error
            );
            std::process::exit(1);
        }
    }
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .filter(|value| !value.starts_with("--"))
        .map(String::as_str)
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
        if d > 180.0 {
            360.0 - d
        } else {
            d
        }
    };

    let dist_right = angle_dist(0.0);
    let dist_up = angle_dist(90.0);
    let dist_left = angle_dist(180.0);
    let dist_down = angle_dist(270.0);

    let min_dist = dist_right.min(dist_up).min(dist_left).min(dist_down);

    if min_dist <= cardinal_half_angle {
        // Pure cardinal zone
        if min_dist == dist_right {
            right = true;
        } else if min_dist == dist_up {
            up = true;
        } else if min_dist == dist_left {
            left = true;
        } else {
            down = true;
        }
    } else if push_strength >= diagonal_min_strength {
        // Diagonal zone with sufficient push
        if angle > 0.0 && angle < 90.0 {
            right = true;
            up = true;
        } else if angle > 90.0 && angle < 180.0 {
            left = true;
            up = true;
        } else if angle > 180.0 && angle < 270.0 {
            left = true;
            down = true;
        } else {
            right = true;
            down = true;
        }
    } else {
        // Diagonal zone but not pushed hard enough — snap to nearest cardinal
        if min_dist == dist_right {
            right = true;
        } else if min_dist == dist_up {
            up = true;
        } else if min_dist == dist_left {
            left = true;
        } else {
            down = true;
        }
    }

    // SOCD cleaning: prevent simultaneous opposite directions
    if up && down {
        up = false;
        down = false;
    }
    if left && right {
        left = false;
        right = false;
    }

    prev_state.up = up;
    prev_state.down = down;
    prev_state.left = left;
    prev_state.right = right;

    (up, down, left, right)
}

fn handle_input(
    window: &Window,
    bus: &mut Bus,
    gilrs: &mut Option<Gilrs>,
    frame_counter: u32,
    input_bindings: &InputBindings,
    stick_state_p1: &mut StickState,
    stick_state_p2: &mut StickState,
) -> (bool, bool, bool, bool) {
    let keys = window.get_keys();
    let turbo_active = (frame_counter / 2).is_multiple_of(2); // ~15Hz: ON 2 frames, OFF 2 frames

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

    let mut p1_up = p1_key_up.is_some_and(|k| keys.contains(&k));
    let mut p1_down = p1_key_down.is_some_and(|k| keys.contains(&k));
    let mut p1_left = p1_key_left.is_some_and(|k| keys.contains(&k));
    let mut p1_right = p1_key_right.is_some_and(|k| keys.contains(&k));
    let mut p1_a = p1_key_a.is_some_and(|k| keys.contains(&k));
    let mut p1_b = p1_key_b.is_some_and(|k| keys.contains(&k));
    let mut p1_start = p1_key_start.is_some_and(|k| keys.contains(&k));
    let mut p1_select = p1_key_select.is_some_and(|k| keys.contains(&k));
    let mut l_trigger = false;
    let mut r_trigger = false;

    // P1 turbo buttons
    if p1_key_turbo_a.is_some_and(|k| keys.contains(&k)) && turbo_active {
        p1_a = true;
    }
    if p1_key_turbo_b.is_some_and(|k| keys.contains(&k)) && turbo_active {
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

    let mut p2_up = p2_key_up.is_some_and(|k| keys.contains(&k));
    let mut p2_down = p2_key_down.is_some_and(|k| keys.contains(&k));
    let mut p2_left = p2_key_left.is_some_and(|k| keys.contains(&k));
    let mut p2_right = p2_key_right.is_some_and(|k| keys.contains(&k));
    let mut p2_a = p2_key_a.is_some_and(|k| keys.contains(&k));
    let mut p2_b = p2_key_b.is_some_and(|k| keys.contains(&k));
    let mut p2_start = p2_key_start.is_some_and(|k| keys.contains(&k));
    let mut p2_select = p2_key_select.is_some_and(|k| keys.contains(&k));

    // P2 turbo buttons
    if p2_key_turbo_a.is_some_and(|k| keys.contains(&k)) && turbo_active {
        p2_a = true;
    }
    if p2_key_turbo_b.is_some_and(|k| keys.contains(&k)) && turbo_active {
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
            let (s_up, s_down, s_left, s_right) =
                stick_to_dpad(stick_x, stick_y, ctrl1.deadzone, stick_state_p1);
            p1_up |= s_up;
            p1_down |= s_down;
            p1_left |= s_left;
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
            l_trigger |=
                gamepad.is_pressed(Button::LeftTrigger) || gamepad.is_pressed(Button::LeftTrigger2);
            r_trigger |= gamepad.is_pressed(Button::RightTrigger)
                || gamepad.is_pressed(Button::RightTrigger2);
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
            let (s_up, s_down, s_left, s_right) =
                stick_to_dpad(stick_x, stick_y, ctrl2.deadzone, stick_state_p2);
            p2_up |= s_up;
            p2_down |= s_down;
            p2_left |= s_left;
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
    bus.joypad1
        .set_button_pressed(JoypadButton::Select, p1_select);
    bus.joypad1
        .set_button_pressed(JoypadButton::Start, p1_start);
    bus.joypad1.set_button_pressed(JoypadButton::Up, p1_up);
    bus.joypad1.set_button_pressed(JoypadButton::Down, p1_down);
    bus.joypad1.set_button_pressed(JoypadButton::Left, p1_left);
    bus.joypad1
        .set_button_pressed(JoypadButton::Right, p1_right);

    bus.joypad2.set_button_pressed(JoypadButton::A, p2_a);
    bus.joypad2.set_button_pressed(JoypadButton::B, p2_b);
    bus.joypad2
        .set_button_pressed(JoypadButton::Select, p2_select);
    bus.joypad2
        .set_button_pressed(JoypadButton::Start, p2_start);
    bus.joypad2.set_button_pressed(JoypadButton::Up, p2_up);
    bus.joypad2.set_button_pressed(JoypadButton::Down, p2_down);
    bus.joypad2.set_button_pressed(JoypadButton::Left, p2_left);
    bus.joypad2
        .set_button_pressed(JoypadButton::Right, p2_right);

    (p1_start, p1_select, l_trigger, r_trigger)
}

#[derive(Clone, Copy)]
struct PixelRect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl PixelRect {
    #[inline]
    fn x2(self) -> usize {
        self.x + self.w
    }

    #[inline]
    fn y2(self) -> usize {
        self.y + self.h
    }
}

#[inline]
fn rgb(r: u32, g: u32, b: u32) -> u32 {
    (r.min(255) << 16) | (g.min(255) << 8) | b.min(255)
}

#[inline]
fn color_channel(color: u32, shift: u32) -> u32 {
    (color >> shift) & 0xFF
}

#[inline]
fn adjust_color(color: u32, delta: i32) -> u32 {
    let adjust = |channel: u32| -> u32 { (channel as i32 + delta).clamp(0, 255) as u32 };
    rgb(
        adjust(color_channel(color, 16)),
        adjust(color_channel(color, 8)),
        adjust(color_channel(color, 0)),
    )
}

#[inline]
fn blend_color(color: u32, target: u32, alpha: f32) -> u32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let blend = |src: u32, dst: u32| -> u32 {
        (src as f32 * (1.0 - alpha) + dst as f32 * alpha).round() as u32
    };
    rgb(
        blend(color_channel(color, 16), color_channel(target, 16)),
        blend(color_channel(color, 8), color_channel(target, 8)),
        blend(color_channel(color, 0), color_channel(target, 0)),
    )
}

#[inline]
fn write_pixel(frame: &mut [u32], x: usize, y: usize, color: u32) {
    if x < WINDOW_WIDTH {
        let idx = y * WINDOW_WIDTH + x;
        if idx < frame.len() {
            frame[idx] = color;
        }
    }
}

#[inline]
fn rounded_rect_contains(lx: usize, ly: usize, w: usize, h: usize, radius: usize) -> bool {
    let radius = radius.min(w / 2).min(h / 2);
    if radius == 0 {
        return true;
    }

    let left = lx < radius;
    let right = lx >= w - radius;
    let top = ly < radius;
    let bottom = ly >= h - radius;

    if (left || right) && (top || bottom) {
        let cx = if left { radius } else { w - 1 - radius };
        let cy = if top { radius } else { h - 1 - radius };
        let dx = lx.abs_diff(cx);
        let dy = ly.abs_diff(cy);
        dx * dx + dy * dy <= radius * radius
    } else {
        true
    }
}

fn draw_rounded_rect<F>(frame: &mut [u32], rect: PixelRect, radius: usize, mut color_at: F)
where
    F: FnMut(usize, usize) -> u32,
{
    let height = frame.len() / WINDOW_WIDTH;
    let y_end = rect.y2().min(height);
    let x_end = rect.x2().min(WINDOW_WIDTH);
    for y in rect.y..y_end {
        for x in rect.x..x_end {
            if rounded_rect_contains(x - rect.x, y - rect.y, rect.w, rect.h, radius) {
                frame[y * WINDOW_WIDTH + x] = color_at(x, y);
            }
        }
    }
}

fn darken_rounded_rect(frame: &mut [u32], rect: PixelRect, radius: usize, alpha: f32) {
    let height = frame.len() / WINDOW_WIDTH;
    let y_end = rect.y2().min(height);
    let x_end = rect.x2().min(WINDOW_WIDTH);
    for y in rect.y..y_end {
        for x in rect.x..x_end {
            if rounded_rect_contains(x - rect.x, y - rect.y, rect.w, rect.h, radius) {
                let idx = y * WINDOW_WIDTH + x;
                frame[idx] = blend_color(frame[idx], 0x000000, alpha);
            }
        }
    }
}

fn draw_hline(frame: &mut [u32], x1: usize, x2: usize, y: usize, color: u32) {
    if y >= frame.len() / WINDOW_WIDTH {
        return;
    }
    for x in x1..x2.min(WINDOW_WIDTH) {
        frame[y * WINDOW_WIDTH + x] = color;
    }
}

fn draw_vline(frame: &mut [u32], x: usize, y1: usize, y2: usize, color: u32) {
    if x >= WINDOW_WIDTH {
        return;
    }
    let height = frame.len() / WINDOW_WIDTH;
    for y in y1..y2.min(height) {
        frame[y * WINDOW_WIDTH + x] = color;
    }
}

fn draw_speaker_grille(frame: &mut [u32], rect: PixelRect) {
    let well = PixelRect {
        x: rect.x.saturating_sub(8),
        y: rect.y.saturating_sub(6),
        w: rect.w + 16,
        h: rect.h + 12,
    };
    draw_rounded_rect(frame, well, 5, |x, y| {
        let lx = x - well.x;
        let ly = y - well.y;
        let edge_x = (lx.min(well.w.saturating_sub(1) - lx)) as f32;
        let edge_y = (ly.min(well.h.saturating_sub(1) - ly)) as f32;
        let edge = edge_x.min(edge_y);
        let lip = ((5.0 - edge).max(0.0) * 3.0) as i32;
        let top_light = if ly < 3 { 18 } else { 0 };
        let bottom_shadow = if ly + 4 >= well.h { -18 } else { 0 };
        adjust_color(0x181A1B, top_light + lip + bottom_shadow)
    });
    draw_hline(frame, well.x + 8, well.x2() - 8, well.y + 1, 0x3B3E40);
    draw_hline(frame, well.x + 8, well.x2() - 8, well.y2() - 2, 0x050606);

    let mirror = rect.x > WINDOW_WIDTH / 2;
    for y in rect.y..rect.y2().min(frame.len() / WINDOW_WIDTH) {
        let ly = y - rect.y;
        let taper = ly * 28 / rect.h.max(1);
        let x1 = if mirror { rect.x + taper } else { rect.x };
        let x2 = if mirror {
            rect.x2()
        } else {
            rect.x2().saturating_sub(taper)
        };
        for x in x1..x2.min(WINDOW_WIDTH) {
            let shade = if ly < 2 {
                0x2F3234
            } else if ly + 2 >= rect.h {
                0x070809
            } else {
                0x1F2224
            };
            frame[y * WINDOW_WIDTH + x] = shade;
        }
    }

    for row in 0..8usize {
        let y = rect.y + 5 + row * 4;
        let taper = (y - rect.y) * 28 / rect.h.max(1);
        let x1 = if mirror {
            rect.x + taper + 4
        } else {
            rect.x + 5
        };
        let x2 = if mirror {
            rect.x2() - 5
        } else {
            rect.x2().saturating_sub(taper + 4)
        };
        draw_hline(frame, x1, x2, y, 0x0B0C0D);
        draw_hline(frame, x1, x2, y + 1, 0x17191A);
        if y > rect.y {
            draw_hline(frame, x1, x2, y - 1, 0x3A3C3E);
        }
    }
}

fn draw_front_button(frame: &mut [u32], rect: PixelRect, base: u32) {
    let cx = rect.x as f32 + rect.w as f32 * 0.5;
    let cy = rect.y as f32 + rect.h as f32 * 0.5;
    let outer_radius = rect.w.min(rect.h) as f32 * 0.5;
    let inner_radius = outer_radius - 1.15;
    let height = frame.len() / WINDOW_WIDTH;

    for y in rect.y.saturating_sub(1)..(rect.y2() + 1).min(height) {
        for x in rect.x.saturating_sub(1)..(rect.x2() + 1).min(WINDOW_WIDTH) {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - cx;
            let dy = py - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let outer_alpha = (outer_radius + 0.5 - dist).clamp(0.0, 1.0);
            if outer_alpha <= 0.0 {
                continue;
            }

            let idx = y * WINDOW_WIDTH + x;
            frame[idx] = blend_color(frame[idx], 0x0D0F10, outer_alpha);

            let inner_alpha = (inner_radius + 0.5 - dist).clamp(0.0, 1.0);
            if inner_alpha > 0.0 {
                let top_lift = ((cy - py).max(0.0) * 4.0) as i32;
                let rim = (dist / inner_radius.max(0.1) * 10.0) as i32;
                let button = adjust_color(base, top_lift - rim);
                frame[idx] = blend_color(frame[idx], button, inner_alpha);
            }
        }
    }
}

fn draw_led(frame: &mut [u32], cx: usize, cy: usize) {
    let cx = cx as f32 + 0.5;
    let cy = cy as f32 + 0.5;
    let height = frame.len() / WINDOW_WIDTH;
    let glow_radius = 10.0f32;
    let outer_radius = 5.5f32;
    let lens_radius = 4.25f32;
    let min_x = (cx - glow_radius - 1.0).max(0.0) as usize;
    let max_x = (cx + glow_radius + 2.0).min(WINDOW_WIDTH as f32) as usize;
    let min_y = (cy - glow_radius - 1.0).max(0.0) as usize;
    let max_y = (cy + glow_radius + 2.0).min(height as f32) as usize;

    for y in min_y..max_y {
        for x in min_x..max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - cx;
            let dy = py - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < glow_radius {
                let glow = (1.0 - dist / glow_radius).powf(1.7) * 0.28;
                let idx = y * WINDOW_WIDTH + x;
                frame[idx] = blend_color(frame[idx], 0x19E35A, glow);
            }
        }
    }

    for y in min_y..max_y {
        for x in min_x..max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - cx;
            let dy = py - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let outer_alpha = (outer_radius + 0.5 - dist).clamp(0.0, 1.0);
            if outer_alpha <= 0.0 {
                continue;
            }

            let idx = y * WINDOW_WIDTH + x;
            frame[idx] = blend_color(frame[idx], 0x0B120D, outer_alpha);

            let lens_alpha = (lens_radius + 0.5 - dist).clamp(0.0, 1.0);
            if lens_alpha > 0.0 {
                let intensity = (1.0 - dist / lens_radius).clamp(0.0, 1.0);
                let highlight = if px < cx && py < cy { 18 } else { 0 };
                let lens = rgb(
                    (32.0 + 82.0 * intensity) as u32 + highlight,
                    (172.0 + 68.0 * intensity) as u32 + highlight,
                    (76.0 + 48.0 * intensity) as u32 + highlight / 2,
                );
                frame[idx] = blend_color(frame[idx], lens, lens_alpha);
            }
        }
    }
}

fn draw_scaled_text(
    frame: &mut [u32],
    text: &str,
    start_x: usize,
    start_y: usize,
    color: u32,
    stride: usize,
    scale: usize,
) {
    let mut cursor_x = start_x;
    for ch in text.chars() {
        let glyph = get_small_glyph(ch);
        for (row, &bits) in glyph.iter().enumerate() {
            for col in 0..3usize {
                if bits & (0b100 >> col) != 0 {
                    let px = cursor_x + col * scale;
                    let py = start_y + row * scale;
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let x = px + sx;
                            let y = py + sy;
                            if x < stride && y * stride + x < frame.len() {
                                frame[y * stride + x] = color;
                            }
                        }
                    }
                }
            }
        }
        cursor_x += 4 * scale;
    }
}

fn build_tv_frame(frame: &mut Vec<u32>) {
    frame.resize(WINDOW_WIDTH * WINDOW_HEIGHT, 0);

    for y in 0..TV_HEIGHT {
        for x in 0..WINDOW_WIDTH {
            let nx = x as f32 / WINDOW_WIDTH as f32 - 0.5;
            let ny = y as f32 / TV_HEIGHT as f32 - 0.38;
            let vignette = ((nx * nx + ny * ny) * 28.0).min(18.0) as i32;
            let vertical = (y as f32 / TV_HEIGHT as f32 * 10.0) as i32;
            let noise =
                ((x.wrapping_mul(7) ^ y.wrapping_mul(13) ^ (x + y).wrapping_mul(3)) % 5) as i32 - 2;
            let shade = vignette + vertical;
            let r = (196 - shade + noise).clamp(0, 255) as u32;
            let g = (198 - shade + noise).clamp(0, 255) as u32;
            let b = (193 - shade + noise).clamp(0, 255) as u32;
            frame[y * WINDOW_WIDTH + x] = rgb(r, g, b);
        }
    }

    let cabinet_w = 1054usize;
    let cabinet_x = (WINDOW_WIDTH - cabinet_w) / 2;
    let body = PixelRect {
        x: cabinet_x,
        y: 14,
        w: cabinet_w,
        h: TV_HEIGHT - 26,
    };

    for layer in (0..88usize).rev() {
        let spread = layer / 2;
        let shadow = PixelRect {
            x: body.x + 26 + spread / 2,
            y: body.y + 34 + spread / 3,
            w: body.w + 64 + layer,
            h: body.h + 36 + spread,
        };
        let alpha = 0.0005 + (88 - layer) as f32 * 0.00009;
        darken_rounded_rect(frame, shadow, 54 + spread, alpha);
    }

    for layer in (0..38usize).rev() {
        let spread = layer / 2;
        let shadow = PixelRect {
            x: body.x.saturating_sub(12 + spread),
            y: body.y + 9 + spread / 2,
            w: body.w + 24 + layer,
            h: body.h + 18 + spread,
        };
        let alpha = 0.007 + (38 - layer) as f32 * 0.0018;
        darken_rounded_rect(frame, shadow, 34 + spread, alpha);
    }

    for layer in (0..20usize).rev() {
        let shadow = PixelRect {
            x: body.x + 18 + layer,
            y: body.y2().saturating_sub(14) + layer / 4,
            w: body.w.saturating_sub(36 + layer * 2),
            h: 14 + layer / 2,
        };
        let alpha = 0.015 + (20 - layer) as f32 * 0.003;
        darken_rounded_rect(frame, shadow, 14 + layer / 2, alpha);
    }

    for layer in (0..18usize).rev() {
        let shadow = PixelRect {
            x: body.x + 2 + layer / 2,
            y: body.y + 10 + layer,
            w: body.w.saturating_sub(layer * 2),
            h: body.h.saturating_sub(layer / 2),
        };
        let alpha = 0.015 + (18 - layer) as f32 * 0.006;
        darken_rounded_rect(frame, shadow, 26 + layer, alpha);
    }

    draw_rounded_rect(frame, body, 27, |x, y| {
        let lx = x - body.x;
        let ly = y - body.y;
        let fx = lx as f32 / body.w as f32;
        let fy = ly as f32 / body.h as f32;
        let edge = (fx - 0.5).abs() * 2.0;
        let top_light = (1.0 - fy).max(0.0) * 18.0;
        let center_lift = (1.0 - edge).max(0.0) * 7.0;
        let lower_shadow = fy * 20.0;
        let side_shadow = edge * 18.0;
        let texture =
            ((x.wrapping_mul(31) ^ y.wrapping_mul(17) ^ (x / 9).wrapping_mul(23)) % 4) as f32;
        let base = (34.0 + top_light + center_lift - lower_shadow - side_shadow + texture)
            .clamp(8.0, 68.0);
        rgb(base as u32, (base + 1.0) as u32, (base + 2.0) as u32)
    });

    draw_hline(frame, body.x + 34, body.x2() - 34, body.y + 2, 0x65696A);
    draw_hline(frame, body.x + 38, body.x2() - 38, body.y + 3, 0x4B4E50);
    draw_hline(frame, body.x + 42, body.x2() - 42, body.y + 9, 0x252728);
    draw_hline(frame, body.x + 36, body.x2() - 36, body.y2() - 4, 0x090A0A);
    draw_hline(frame, body.x + 42, body.x2() - 42, body.y2() - 3, 0x202123);
    draw_vline(frame, body.x + 2, body.y + 38, body.y2() - 40, 0x4C4F50);
    draw_vline(frame, body.x + 3, body.y + 40, body.y2() - 42, 0x2A2C2D);
    draw_vline(frame, body.x2() - 3, body.y + 38, body.y2() - 40, 0x070707);
    draw_vline(frame, body.x2() - 4, body.y + 40, body.y2() - 42, 0x151617);
    for inset in 0..5usize {
        let shade = adjust_color(0x1A1D1E, -(inset as i32 * 3));
        draw_vline(
            frame,
            body.x2().saturating_sub(11 + inset),
            body.y + 52 + inset,
            body.y2().saturating_sub(52 + inset),
            shade,
        );
    }
    draw_hline(frame, body.x + 54, body.x2() - 74, body.y + 18, 0x35393A);
    draw_hline(frame, body.x + 78, body.x2() - 94, body.y + 19, 0x111314);
    draw_hline(frame, body.x + 44, body.x2() - 58, body.y2() - 18, 0x040505);
    draw_hline(frame, body.x + 82, body.x2() - 96, body.y2() - 17, 0x141617);

    let top_hood = PixelRect {
        x: SCREEN_X - 50,
        y: body.y + 22,
        w: SCREEN_W + 100,
        h: SCREEN_Y.saturating_sub(body.y + 30).max(22),
    };
    draw_rounded_rect(frame, top_hood, 13, |x, y| {
        let ly = y - top_hood.y;
        let fx = (x - top_hood.x) as f32 / top_hood.w as f32;
        let side_falloff = (fx - 0.5).abs() * 2.0;
        let highlight = if ly < 3 { 22.0 - ly as f32 * 4.0 } else { 0.0 };
        let base = (26.0 + highlight - side_falloff * 10.0 - ly as f32 * 0.45).clamp(6.0, 54.0);
        rgb(base as u32, (base + 1.0) as u32, (base + 2.0) as u32)
    });
    draw_hline(
        frame,
        top_hood.x + 18,
        top_hood.x2() - 18,
        top_hood.y + 2,
        0x53585A,
    );
    draw_hline(
        frame,
        top_hood.x + 34,
        top_hood.x2() - 34,
        top_hood.y2() - 2,
        0x060707,
    );

    let left_cheek = PixelRect {
        x: body.x + 12,
        y: SCREEN_Y - 10,
        w: SCREEN_X.saturating_sub(body.x + 28),
        h: SCREEN_H + 58,
    };
    let right_cheek = PixelRect {
        x: SCREEN_X + SCREEN_W + 16,
        y: SCREEN_Y - 10,
        w: body.x2().saturating_sub(SCREEN_X + SCREEN_W + 28),
        h: SCREEN_H + 58,
    };
    for (cheek, mirror) in [(left_cheek, false), (right_cheek, true)] {
        draw_rounded_rect(frame, cheek, 18, |x, y| {
            let lx = x - cheek.x;
            let ly = y - cheek.y;
            let edge = if mirror {
                lx as f32 / cheek.w.max(1) as f32
            } else {
                1.0 - lx as f32 / cheek.w.max(1) as f32
            };
            let vertical = ly as f32 / cheek.h.max(1) as f32;
            let texture = ((x.wrapping_mul(11) ^ y.wrapping_mul(19)) % 3) as f32;
            let base = (18.0 + edge * 15.0 - vertical * 8.0 + texture).clamp(4.0, 39.0);
            rgb(base as u32, (base + 1.0) as u32, (base + 2.0) as u32)
        });
        let edge_x = if mirror {
            cheek.x
        } else {
            cheek.x2().saturating_sub(2)
        };
        draw_vline(
            frame,
            edge_x,
            cheek.y + 18,
            cheek.y2().saturating_sub(18),
            0x080909,
        );
        draw_vline(
            frame,
            if mirror {
                cheek.x + 2
            } else {
                cheek.x2().saturating_sub(4)
            },
            cheek.y + 22,
            cheek.y2().saturating_sub(22),
            0x2E3234,
        );
    }

    let bezel = PixelRect {
        x: body.x + 25,
        y: 31,
        w: body.w - 50,
        h: SCREEN_Y + SCREEN_H + 17 - 31,
    };
    draw_rounded_rect(frame, bezel, 13, |x, y| {
        let lx = x - bezel.x;
        let ly = y - bezel.y;
        let fx = lx as f32 / bezel.w as f32;
        let fy = ly as f32 / bezel.h as f32;
        let edge = (fx - 0.5).abs() * 2.0;
        let upper = (1.0 - fy).max(0.0) * 9.0;
        let recess = if (SCREEN_X..SCREEN_X + SCREEN_W).contains(&x)
            && (SCREEN_Y..SCREEN_Y + SCREEN_H).contains(&y)
        {
            -20.0
        } else {
            0.0
        };
        let base = (18.0 + upper - edge * 6.0 + recess).clamp(2.0, 34.0);
        rgb(base as u32, (base + 1.0) as u32, (base + 2.0) as u32)
    });

    let tube_recess = PixelRect {
        x: SCREEN_X - 24,
        y: SCREEN_Y - 18,
        w: SCREEN_W + 48,
        h: SCREEN_H + 39,
    };
    draw_rounded_rect(frame, tube_recess, 12, |x, y| {
        let lx = x - tube_recess.x;
        let ly = y - tube_recess.y;
        let fx = lx as f32 / tube_recess.w as f32;
        let fy = ly as f32 / tube_recess.h as f32;
        let top_lift = (1.0 - fy).max(0.0) * 8.0;
        let side_lift = (1.0 - fx).max(0.0) * 4.0;
        let sink = fy * 8.0 + fx * 3.0;
        let base = (13.0 + top_lift + side_lift - sink).clamp(1.0, 27.0);
        rgb(base as u32, (base + 1.0) as u32, (base + 2.0) as u32)
    });
    draw_hline(
        frame,
        tube_recess.x + 22,
        tube_recess.x2() - 22,
        tube_recess.y + 2,
        0x303436,
    );
    draw_hline(
        frame,
        tube_recess.x + 22,
        tube_recess.x2() - 22,
        tube_recess.y2() - 3,
        0x030303,
    );

    let glass_lip = PixelRect {
        x: SCREEN_X - 8,
        y: SCREEN_Y - 8,
        w: SCREEN_W + 16,
        h: SCREEN_H + 16,
    };
    draw_rounded_rect(frame, glass_lip, 8, |x, y| {
        let lx = x - glass_lip.x;
        let ly = y - glass_lip.y;
        let fx = lx as f32 / glass_lip.w as f32;
        let fy = ly as f32 / glass_lip.h as f32;
        let top = if ly < 6 { (6 - ly) as f32 * 1.5 } else { 0.0 };
        let left = if lx < 6 { (6 - lx) as f32 * 0.7 } else { 0.0 };
        let base = (5.0 + top + left - fy * 6.0 - fx * 2.0).clamp(0.0, 18.0);
        rgb(base as u32, base as u32, (base + 1.0) as u32)
    });
    draw_hline(
        frame,
        SCREEN_X + 8,
        SCREEN_X + SCREEN_W - 8,
        SCREEN_Y - 2,
        0x181A1B,
    );
    draw_hline(
        frame,
        SCREEN_X + 10,
        SCREEN_X + SCREEN_W - 10,
        SCREEN_Y + SCREEN_H + 1,
        0x010101,
    );

    for y in (SCREEN_Y + SCREEN_H + 13)..(SCREEN_Y + SCREEN_H + 18) {
        let shade = 0x2A2D2E - ((y - (SCREEN_Y + SCREEN_H + 13)) as u32 * 0x020202);
        draw_hline(frame, body.x + 16, body.x2() - 16, y, shade);
    }

    let panel = PixelRect {
        x: body.x + 17,
        y: SCREEN_Y + SCREEN_H + 19,
        w: body.w - 34,
        h: 75,
    };
    draw_rounded_rect(frame, panel, 8, |x, y| {
        let ly = y - panel.y;
        let fx = (x - panel.x) as f32 / panel.w as f32;
        let center = (1.0 - (fx - 0.5).abs() * 2.0) * 5.0;
        let top = if ly < 5 { 13.0 } else { 0.0 };
        let bottom = if ly + 5 >= panel.h { -14.0 } else { 0.0 };
        let base = (24.0 + center + top + bottom).clamp(9.0, 44.0);
        rgb(base as u32, (base + 1.0) as u32, (base + 2.0) as u32)
    });
    draw_hline(frame, panel.x + 10, panel.x2() - 10, panel.y + 1, 0x4A4E50);
    draw_hline(
        frame,
        panel.x + 10,
        panel.x2() - 10,
        panel.y2() - 2,
        0x070707,
    );
    draw_hline(frame, panel.x + 16, panel.x2() - 16, panel.y + 7, 0x35383A);
    draw_hline(frame, panel.x + 16, panel.x2() - 16, panel.y + 9, 0x090A0A);

    let center_controls = PixelRect {
        x: WINDOW_WIDTH / 2 - 96,
        y: panel.y + 30,
        w: 244,
        h: 35,
    };
    draw_rounded_rect(frame, center_controls, 6, |x, y| {
        let lx = x - center_controls.x;
        let ly = y - center_controls.y;
        let fx = lx as f32 / center_controls.w as f32;
        let top = if ly < 3 { 10.0 } else { 0.0 };
        let bottom = if ly + 3 >= center_controls.h {
            -11.0
        } else {
            0.0
        };
        let side = (fx - 0.5).abs() * -4.0;
        let base = (17.0 + top + bottom + side).clamp(5.0, 31.0);
        rgb(base as u32, (base + 1.0) as u32, (base + 2.0) as u32)
    });
    draw_hline(
        frame,
        center_controls.x + 8,
        center_controls.x2() - 8,
        center_controls.y + 1,
        0x323638,
    );
    draw_hline(
        frame,
        center_controls.x + 8,
        center_controls.x2() - 8,
        center_controls.y2() - 2,
        0x060606,
    );

    draw_speaker_grille(
        frame,
        PixelRect {
            x: panel.x + 2,
            y: panel.y + 12,
            w: 308,
            h: 47,
        },
    );
    draw_speaker_grille(
        frame,
        PixelRect {
            x: panel.x2() - 310,
            y: panel.y + 12,
            w: 308,
            h: 47,
        },
    );

    let logo_scale = 2;
    let logo_w = "OXIDENES".len() * 4 * logo_scale - logo_scale;
    let logo_x = WINDOW_WIDTH / 2 - logo_w / 2;
    draw_scaled_text(
        frame,
        "OXIDENES",
        logo_x,
        panel.y + 12,
        0xBAC1C8,
        WINDOW_WIDTH,
        logo_scale,
    );
    write_pixel(frame, logo_x + 28, panel.y + 16, 0x4E88D8);
    write_pixel(frame, logo_x + 29, panel.y + 15, 0x9EC7FF);

    for i in 0..6usize {
        draw_front_button(
            frame,
            PixelRect {
                x: WINDOW_WIDTH / 2 - 54 + i * 16,
                y: panel.y + 42,
                w: 9,
                h: 9,
            },
            0x3B3F42,
        );
    }

    draw_rounded_rect(
        frame,
        PixelRect {
            x: WINDOW_WIDTH / 2 + 88,
            y: panel.y + 38,
            w: 48,
            h: 17,
        },
        2,
        |x, y| {
            let lx = x - (WINDOW_WIDTH / 2 + 88);
            let ly = y - (panel.y + 38);
            let shine = if ly < 3 && lx > 4 && lx < 24 { 10 } else { 0 };
            adjust_color(0x050607, shine)
        },
    );
    draw_led(frame, WINDOW_WIDTH / 2 + 178, panel.y + 46);

    let left_foot = PixelRect {
        x: body.x + 35,
        y: TV_HEIGHT - 21,
        w: 210,
        h: 13,
    };
    let right_foot = PixelRect {
        x: body.x2() - 245,
        y: TV_HEIGHT - 21,
        w: 210,
        h: 13,
    };
    for foot in [left_foot, right_foot] {
        draw_rounded_rect(frame, foot, 6, |_, y| {
            let ly = y - foot.y;
            if ly < 2 {
                0x222324
            } else {
                0x050505
            }
        });
        draw_hline(frame, foot.x + 12, foot.x2() - 12, foot.y + 1, 0x323334);
    }
}

#[inline]
fn scale_pixel(pixel: u32, shade: u32) -> u32 {
    rgb(
        (((pixel >> 16) & 0xFF) * shade) >> 8,
        (((pixel >> 8) & 0xFF) * shade) >> 8,
        ((pixel & 0xFF) * shade) >> 8,
    )
}

fn composite_screen_fast(
    result: &mut [u32],
    game_output: &[u32],
    screen_curve_table: &[u32],
    window_width: usize,
) {
    debug_assert!(screen_curve_table.len() >= SCREEN_W * SCREEN_H);

    // Only touch the CRT screen area on the persistent composite buffer.
    // The table gives the tube a slight convex-glass bow without moving the cabinet.
    for src_y in 0..SCREEN_H {
        let dst_row_start = (src_y + SCREEN_Y) * window_width + SCREEN_X;
        let table_row_start = src_y * SCREEN_W;
        for src_x in 0..SCREEN_W {
            let sample = unsafe { *screen_curve_table.get_unchecked(table_row_start + src_x) };
            let src_idx = (sample & SCREEN_CURVE_SRC_MASK) as usize;
            let shade = sample >> SCREEN_CURVE_SRC_BITS;
            let pixel = unsafe { *game_output.get_unchecked(src_idx) };
            let out = if shade == 256 {
                pixel
            } else {
                scale_pixel(pixel, shade)
            };
            unsafe {
                *result.get_unchecked_mut(dst_row_start + src_x) = out;
            }
        }
    }
}

fn build_glare_table() -> Vec<u8> {
    let mut table = vec![0u8; SCREEN_W * SCREEN_H];

    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            let fx = x as f64 / SCREEN_W as f64; // 0..1
            let fy = y as f64 / SCREEN_H as f64; // 0..1
            let nx = fx * 2.0 - 1.0; // -1..1
            let ny = fy * 2.0 - 1.0; // -1..1

            // Edge distance for Fresnel and fading
            let edge_dist = nx.abs().max(ny.abs()).min(1.0);

            let fresnel_t = ((edge_dist - 0.58).max(0.0) / 0.42).min(1.0);
            let fresnel = fresnel_t.powi(3) * (24.0 + (1.0 - ny.abs()) * 10.0);

            // Soft, asymmetric room reflections over convex CRT glass.
            // The curve keeps the screen readable while avoiding a flat LCD-like band.
            let top_curve = 0.115 + (fx - 0.44) * (fx - 0.44) * 0.105;
            let top_width = 0.043 + (fx - 0.36).abs() * 0.018;
            let top_band = (-(((fy - top_curve) / top_width).powi(2)) / 2.0).exp();
            let top_fade = (1.0 - ((fx - 0.40).abs() / 0.52).powi(4)).max(0.0);
            let top_sheen = top_band * top_fade * 52.0;

            let broad_curve = 0.255 - 0.072 * nx * nx;
            let broad_band = (-(((fy - broad_curve) / 0.105).powi(2)) / 2.0).exp();
            let broad_fade = (1.0 - (nx.abs() - 0.78).max(0.0) * 4.5).max(0.0);
            let broad_sheen = broad_band * broad_fade * 18.0;

            let side_left = (-(((fx - 0.065) / 0.045).powi(2)) / 2.0).exp()
                * (-(((fy - 0.42) / 0.50).powi(2)) / 2.0).exp()
                * 18.0;
            let side_right = (-(((fx - 0.935) / 0.05).powi(2)) / 2.0).exp()
                * (-(((fy - 0.40) / 0.48).powi(2)) / 2.0).exp()
                * 12.0;

            let small_glint_x = (fx - 0.63) / 0.18;
            let small_glint_y = (fy - 0.18) / 0.055;
            let small_glint =
                (-(small_glint_x * small_glint_x + small_glint_y * small_glint_y) / 2.0).exp()
                    * 15.0;

            let bottom_glow_t = ((fy - 0.85).max(0.0) / 0.15).min(1.0);
            let bottom_center = (-(nx * nx) * 3.0).exp();
            let bottom = bottom_glow_t * bottom_center * 12.0;

            let total =
                (fresnel + top_sheen + broad_sheen + side_left + side_right + small_glint + bottom)
                    .clamp(0.0, 96.0) as u8;

            // Zero out near border (glass-bezel junction has no glare)
            let in_border = !(4..SCREEN_W - 4).contains(&x) || !(4..SCREEN_H - 4).contains(&y);
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
    if glass_intensity < 20 {
        return table;
    }
    let base_alpha = ((glass_intensity as f64) - 20.0) * 16.0 / 80.0;
    if base_alpha <= 0.0 {
        return table;
    }

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

/// Combined glass effects: tint + specular glare + internal ghost reflection.
/// Single pass over the screen area — eliminates one full buffer traversal.
#[allow(clippy::too_many_arguments)]
fn apply_glass_effects(
    buffer: &mut [u32],
    ghost_source: &[u32],
    glare_table: &[u8],
    thickness_table: &[u16],
    ghost_alpha_table: &[u8],
    window_width: usize,
    glass_intensity: u8,
    do_ghost: bool,
    ghost_stride: usize,
) {
    if glass_intensity == 0 {
        return;
    }

    const CORNER_R: usize = SCREEN_CURVE_CORNER_R;
    let intensity_factor = glass_intensity as u32;
    let tint_strength = intensity_factor * 10 / 100;
    let corner_x_max = SCREEN_W - CORNER_R;
    let corner_y_max = SCREEN_H - CORNER_R;
    let ghost_shift_x: usize = 3;
    let ghost_shift_y: usize = 2;
    let ghost_h = SCREEN_H.saturating_sub(ghost_shift_y);
    let ghost_w = SCREEN_W.saturating_sub(ghost_shift_x);

    // Monomorphize on do_ghost so compiler eliminates dead ghost code
    if do_ghost {
        glass_inner_loop(
            buffer,
            ghost_source,
            glare_table,
            thickness_table,
            ghost_alpha_table,
            window_width,
            intensity_factor,
            tint_strength,
            true,
            corner_x_max,
            corner_y_max,
            ghost_shift_x,
            ghost_shift_y,
            ghost_h,
            ghost_w,
            ghost_stride,
        );
    } else {
        glass_inner_loop(
            buffer,
            ghost_source,
            glare_table,
            thickness_table,
            ghost_alpha_table,
            window_width,
            intensity_factor,
            tint_strength,
            false,
            corner_x_max,
            corner_y_max,
            ghost_shift_x,
            ghost_shift_y,
            ghost_h,
            ghost_w,
            ghost_stride,
        );
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
fn apply_chromatic_aberration(
    buffer: &mut [u32],
    source: &[u32],
    ca_table: &CaTable,
    width: usize,
    height: usize,
) {
    let w = width as i32;
    let h = height as i32;

    for y in 0..height {
        let row = y * width;
        for x in 0..width {
            let (sx, sy) = unsafe { *ca_table.shifts.get_unchecked(row + x) };
            if sx == 0 && sy == 0 {
                continue;
            }

            let r_x = ((x as i32) - sx as i32).clamp(0, w - 1) as usize;
            let r_y = ((y as i32) - sy as i32).clamp(0, h - 1) as usize;
            let b_x = ((x as i32) + sx as i32).clamp(0, w - 1) as usize;
            let b_y = ((y as i32) + sy as i32).clamp(0, h - 1) as usize;

            let r = unsafe { (*source.get_unchecked(r_y * width + r_x) >> 16) & 0xFF };
            let g = unsafe { (*source.get_unchecked(row + x) >> 8) & 0xFF };
            let b = unsafe { *source.get_unchecked(b_y * width + b_x) & 0xFF };

            unsafe {
                *buffer.get_unchecked_mut(row + x) = (r << 16) | (g << 8) | b;
            }
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

#[allow(clippy::ptr_arg)]
fn draw_text(
    frame: &mut Vec<u32>,
    text: &str,
    start_x: usize,
    start_y: usize,
    color: u32,
    stride: usize,
) {
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

#[allow(clippy::ptr_arg)]
fn build_console_overlay(frame: &mut Vec<u32>, tv_h: usize, win_w: usize, win_h: usize) {
    let table_h = (win_h - tv_h).max(1);
    for y in tv_h..win_h {
        for x in 0..win_w {
            let idx = y * win_w + x;
            if idx < frame.len() {
                let dy = y - tv_h;
                let fy = dy as f32 / table_h as f32;
                let top_shadow = if dy < 30 {
                    (30 - dy) as f32 / 30.0
                } else {
                    0.0
                };
                let front_rolloff = ((fy - 0.76).max(0.0) / 0.24).min(1.0);
                let center_lift = (1.0 - (fy - 0.48).abs() * 1.8).max(0.0) * 15.0;

                let plank = x / 168;
                let plank_tint = ((plank.wrapping_mul(29) % 19) as i32) - 9;
                let seam = x % 168;
                let seam_shadow = if !(2..=165).contains(&seam) { -16 } else { 0 };
                let long_grain = if ((x / 21 + y / 5 + plank * 3) % 7) == 0 {
                    -7
                } else {
                    0
                };
                let fine_grain =
                    ((x.wrapping_mul(17) ^ y.wrapping_mul(31) ^ (x / 13).wrapping_mul(11)) % 21)
                        as i32
                        - 10;
                let knot = if ((x + plank * 53) % 271) < 22 && ((dy + x / 37) % 19) < 5 {
                    -10
                } else {
                    0
                };

                let shade = center_lift - top_shadow * 48.0 - front_rolloff * 24.0;
                let r = (92.0 + shade) as i32
                    + plank_tint
                    + seam_shadow
                    + fine_grain
                    + long_grain
                    + knot;
                let g = (50.0 + shade * 0.48) as i32
                    + plank_tint / 2
                    + seam_shadow / 2
                    + fine_grain / 2;
                let b = (25.0 + shade * 0.22) as i32 + plank_tint / 4 + fine_grain / 4;
                frame[idx] = rgb(
                    r.clamp(0, 255) as u32,
                    g.clamp(0, 255) as u32,
                    b.clamp(0, 255) as u32,
                );
            }
        }
    }

    for y in tv_h..(tv_h + 16).min(win_h) {
        let dy = y - tv_h;
        let alpha = (0.52 - dy as f32 * 0.026).max(0.0);
        for x in 28..win_w.saturating_sub(28) {
            let idx = y * win_w + x;
            if idx < frame.len() {
                frame[idx] = blend_color(frame[idx], 0x080503, alpha);
            }
        }
    }

    let back_edge_y = tv_h + 1;
    if back_edge_y < win_h {
        for x in 46..win_w.saturating_sub(46) {
            let idx = back_edge_y * win_w + x;
            if idx < frame.len() {
                frame[idx] = 0x1B0E06;
            }
        }
    }

    let front_lip_y = win_h.saturating_sub(18);
    for y in front_lip_y..win_h {
        let ly = y - front_lip_y;
        let highlight = ly == 0;
        for x in 0..win_w {
            let idx = y * win_w + x;
            if idx < frame.len() {
                if highlight && (60..win_w.saturating_sub(60)).contains(&x) {
                    frame[idx] = blend_color(frame[idx], 0xA76632, 0.46);
                } else {
                    let alpha = 0.10 + ly as f32 * 0.018;
                    frame[idx] = blend_color(frame[idx], 0x2E1609, alpha);
                }
            }
        }
    }
}

/// Read joypad state as a byte (bit layout: A=0, B=1, Select=2, Start=3, Up=4, Down=5, Left=6, Right=7)
fn joypad_to_byte(bus: &Bus, player: u8) -> u8 {
    let jp = if player == 1 {
        &bus.joypad1
    } else {
        &bus.joypad2
    };
    let mut b: u8 = 0;
    if jp.get_button(JoypadButton::A) {
        b |= 0x01;
    }
    if jp.get_button(JoypadButton::B) {
        b |= 0x02;
    }
    if jp.get_button(JoypadButton::Select) {
        b |= 0x04;
    }
    if jp.get_button(JoypadButton::Start) {
        b |= 0x08;
    }
    if jp.get_button(JoypadButton::Up) {
        b |= 0x10;
    }
    if jp.get_button(JoypadButton::Down) {
        b |= 0x20;
    }
    if jp.get_button(JoypadButton::Left) {
        b |= 0x40;
    }
    if jp.get_button(JoypadButton::Right) {
        b |= 0x80;
    }
    b
}

/// Apply a byte of button state onto a joypad
fn byte_to_joypad(bus: &mut Bus, player: u8, buttons: u8) {
    let jp = if player == 1 {
        &mut bus.joypad1
    } else {
        &mut bus.joypad2
    };
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
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| PathBuf::from(p).join(".nes-emulator").join("recordings"))
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".nes-emulator").join("recordings"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Performance overlay helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Write the basic overlay line into `buf` without allocating.
/// Format: `"FPS:60 16.7ms"`
fn fmt_basic_line(buf: &mut String, fps: u32, frame_ms: f32) {
    use std::fmt::Write;
    buf.clear();
    let _ = write!(buf, "FPS:{fps} {frame_ms:.1}ms");
}

/// Write the detailed stage breakdown into `buf` without allocating.
/// Format: `"C:1234 B:567 CS:234 GL:891"` (all values in µs)
fn fmt_detail_line(buf: &mut String, snap: &PerfSnapshot) {
    use std::fmt::Write;
    buf.clear();
    let _ = write!(
        buf,
        "C:{} B:{} CS:{} GL:{}",
        snap.crt_us, snap.bloom_us, snap.composite_us, snap.glass_us,
    );
}

/// Chooses the y-position for the Basic FPS row.
///
/// When the rewind bar is visible, move the FPS line down one row so both
/// overlays remain readable.
fn perf_basic_overlay_y(has_rewind_bar: bool) -> usize {
    SCREEN_Y + if has_rewind_bar { 20 } else { 8 }
}

/// Chooses the y-position for the Detailed metrics row.
///
/// REC/PLAY already occupies the second overlay row, so Detailed metrics move
/// down one line when that transport HUD is visible.
fn perf_detail_overlay_y(has_transport_hud: bool) -> usize {
    SCREEN_Y + if has_transport_hud { 32 } else { 20 }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_state_test_bus() -> Bus {
        let mut rom = vec![0u8; 16 + 16384 + 8192];
        rom[0] = 0x4E;
        rom[1] = 0x45;
        rom[2] = 0x53;
        rom[3] = 0x1A;
        rom[4] = 1;
        rom[5] = 1;
        Bus::new(Cartridge::new(&rom).expect("test ROM should load"))
    }

    fn save_state_bytes_for_test(bus: &Bus, cpu: &Cpu) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"NESSAV02");
        let cpu_state = cpu.save_state();
        data.extend_from_slice(&(cpu_state.len() as u32).to_le_bytes());
        data.extend(cpu_state);
        let bus_state = bus.save_state();
        data.extend_from_slice(&(bus_state.len() as u32).to_le_bytes());
        data.extend(bus_state);
        data
    }

    #[test]
    fn load_state_from_bytes_restores_v02_state() {
        let mut source_bus = make_state_test_bus();
        source_bus.cpu_write(0x0000, 0x42);
        let mut source_cpu = Cpu::new();
        source_cpu.a = 0x99;
        source_cpu.pc = 0x1234;
        let data = save_state_bytes_for_test(&source_bus, &source_cpu);

        let mut target_bus = make_state_test_bus();
        let mut target_cpu = Cpu::new();

        assert!(load_state_from_bytes(
            &mut target_bus,
            &mut target_cpu,
            &data
        ));
        assert_eq!(target_bus.cpu_read(0x0000), 0x42);
        assert_eq!(target_cpu.a, 0x99);
        assert_eq!(target_cpu.pc, 0x1234);
    }

    #[test]
    fn load_state_from_bytes_rejects_truncated_cpu_payload_without_panic() {
        let mut data = Vec::new();
        data.extend_from_slice(b"NESSAV02");
        data.extend_from_slice(&2u32.to_le_bytes());
        data.push(0xAA);

        let mut bus = make_state_test_bus();
        let mut cpu = Cpu::new();
        cpu.a = 0x42;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            load_state_from_bytes(&mut bus, &mut cpu, &data)
        }));

        assert!(result.is_ok(), "truncated save state should not panic");
        assert!(!result.unwrap(), "truncated CPU payload should be rejected");
        assert_eq!(cpu.a, 0x42);
    }

    #[test]
    fn load_state_from_bytes_rejects_truncated_bus_payload_without_panic() {
        let bus_source = make_state_test_bus();
        let cpu_source = Cpu::new();
        let cpu_state = cpu_source.save_state();
        let mut data = Vec::new();
        data.extend_from_slice(b"NESSAV02");
        data.extend_from_slice(&(cpu_state.len() as u32).to_le_bytes());
        data.extend(cpu_state);
        data.extend_from_slice(&64u32.to_le_bytes());
        data.extend_from_slice(&bus_source.save_state()[..8]);

        let mut bus = make_state_test_bus();
        let mut cpu = Cpu::new();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            load_state_from_bytes(&mut bus, &mut cpu, &data)
        }));

        assert!(result.is_ok(), "truncated save state should not panic");
        assert!(!result.unwrap(), "truncated bus payload should be rejected");
    }

    /// Helper: compute scan_muls from scanline_intensity (mirrors crt_filter logic).
    fn make_scan_muls(scanline_intensity: u8) -> [u32; 4] {
        let si = scanline_intensity as u32;
        [
            255,
            255 - (si * 15 / 100),
            255 - (si * 25 / 100),
            255 - si.min(255) * 55 / 100,
        ]
    }

    // ── TEST 1: sv_table has the correct number of entries ──────────────────
    #[test]
    fn sv_table_correct_size() {
        let vignette = build_vignette_table_with_strength(20);
        let sv = build_sv_table(&vignette, 40);
        assert_eq!(
            sv.len(),
            SCREEN_W * SCREEN_H,
            "sv_table must have exactly SCREEN_W * SCREEN_H entries"
        );
    }

    // ── TEST 2: sv_table matches (scan_mul * vig) >> 8 at sampled pixels ───
    #[test]
    fn sv_table_matches_old_formula_sampled() {
        let strength: u8 = 20;
        let scanline_intensity: u8 = 40;
        let vignette = build_vignette_table_with_strength(strength);
        let sv = build_sv_table(&vignette, scanline_intensity);
        let scan_muls = make_scan_muls(scanline_intensity);

        // Sample pixels across all 4 scanline phases and various x positions
        let right = SCREEN_W - 1;
        let mid_x = SCREEN_W / 2;
        let bottom = SCREEN_H - 1;
        let samples: &[(usize, usize)] = &[
            (0, 0),
            (0, mid_x.saturating_sub(1)),
            (0, right),
            (1, 0),
            (1, 200.min(right)),
            (1, right),
            (2, 0),
            (2, mid_x),
            (2, right),
            (3, 0),
            (3, bottom),
            (3, right),
            (100.min(bottom), 300.min(right)),
            (240.min(bottom), 500.min(right)),
            (400.min(bottom), 0),
            (bottom, right),
        ];
        for &(dst_y, dst_x) in samples {
            let idx = dst_y * SCREEN_W + dst_x;
            let vig = vignette[idx] as u32;
            let scan_mul = scan_muls[dst_y % 4];
            let expected = ((scan_mul * vig) >> 8) as u8;
            assert_eq!(
                sv[idx], expected,
                "sv_table mismatch at ({dst_y}, {dst_x}): got {}, expected {expected}",
                sv[idx]
            );
        }
    }

    // ── TEST 3: sv_table exhaustive at scanline_intensity = 0 ───────────────
    // When si=0, all scan_muls are 255; sv = (255 * vig) >> 8.
    #[test]
    fn sv_table_zero_intensity_exhaustive() {
        let vignette = build_vignette_table_with_strength(30);
        let sv = build_sv_table(&vignette, 0);
        let scan_muls = make_scan_muls(0);

        for idx in 0..SCREEN_W * SCREEN_H {
            let dst_y = idx / SCREEN_W;
            let vig = vignette[idx] as u32;
            let scan_mul = scan_muls[dst_y % 4];
            let expected = ((scan_mul * vig) >> 8) as u8;
            assert_eq!(sv[idx], expected, "mismatch at idx {idx}");
        }
    }

    // ── TEST 4: sv_table at max scanline intensity (100) ────────────────────
    #[test]
    fn sv_table_max_intensity_sampled() {
        let vignette = build_vignette_table_with_strength(50);
        let sv = build_sv_table(&vignette, 100);
        let scan_muls = make_scan_muls(100);

        for dst_y in [
            0usize,
            1,
            2,
            3,
            SCREEN_H - 4,
            SCREEN_H - 3,
            SCREEN_H - 2,
            SCREEN_H - 1,
        ] {
            for dst_x in [0usize, 1, SCREEN_W / 2, SCREEN_W - 2, SCREEN_W - 1] {
                let idx = dst_y * SCREEN_W + dst_x;
                let vig = vignette[idx] as u32;
                let scan_mul = scan_muls[dst_y % 4];
                let expected = ((scan_mul * vig) >> 8) as u8;
                assert_eq!(
                    sv[idx], expected,
                    "sv_table max-intensity mismatch at ({dst_y}, {dst_x})"
                );
            }
        }
    }

    // ── TEST: PerfOverlayLevel cycling ──────────────────────────────────────
    #[test]
    fn perf_overlay_level_cycles_correctly() {
        assert_eq!(
            PerfOverlayLevel::Off.next(),
            PerfOverlayLevel::Basic,
            "Off -> Basic"
        );
        assert_eq!(
            PerfOverlayLevel::Basic.next(),
            PerfOverlayLevel::Detailed,
            "Basic -> Detailed"
        );
        assert_eq!(
            PerfOverlayLevel::Detailed.next(),
            PerfOverlayLevel::Off,
            "Detailed -> Off (wraps)"
        );
        // Full round-trip
        let level = PerfOverlayLevel::Off;
        let level = level.next(); // Basic
        let level = level.next(); // Detailed
        let level = level.next(); // Off again
        assert_eq!(level, PerfOverlayLevel::Off, "three cycles returns to Off");
    }

    // ── TEST: fmt_basic_line formatting and string reuse ────────────────────
    #[test]
    fn fmt_basic_line_correct() {
        let mut buf = String::new();
        fmt_basic_line(&mut buf, 60, 16.7);
        assert_eq!(buf, "FPS:60 16.7ms");

        // Reuses same allocation — clears then writes
        fmt_basic_line(&mut buf, 30, 33.3);
        assert_eq!(buf, "FPS:30 33.3ms");

        // Edge: 0 fps, 0ms
        fmt_basic_line(&mut buf, 0, 0.0);
        assert_eq!(buf, "FPS:0 0.0ms");
    }

    // ── TEST: Off-to-visible overlay transition must signal fps reset ────────
    // This test was written BEFORE the fix existed.
    // Root cause: fps_timer was not reset when F10 cycled Off->Basic, causing
    // stale elapsed time to produce garbage values on first display (e.g. "FPS:1 60000.0ms").
    #[test]
    fn perf_overlay_off_to_visible_requires_fps_reset() {
        // Off -> Basic is the transition where stale time accumulates
        assert!(
            should_reset_fps_on_transition(PerfOverlayLevel::Off, PerfOverlayLevel::Basic),
            "Off->Basic must signal fps reset to prevent stale elapsed time"
        );
        // Basic -> Detailed must NOT reset: fps_timer was already running
        assert!(
            !should_reset_fps_on_transition(PerfOverlayLevel::Basic, PerfOverlayLevel::Detailed),
            "Basic->Detailed must not reset (timer was already running)"
        );
        // Turning overlay off does not require reset
        assert!(
            !should_reset_fps_on_transition(PerfOverlayLevel::Basic, PerfOverlayLevel::Off),
            "Basic->Off must not reset"
        );
        assert!(
            !should_reset_fps_on_transition(PerfOverlayLevel::Detailed, PerfOverlayLevel::Off),
            "Detailed->Off must not reset"
        );
        // Off -> Off is a no-op
        assert!(
            !should_reset_fps_on_transition(PerfOverlayLevel::Off, PerfOverlayLevel::Off),
            "Off->Off must not reset"
        );
    }

    // ── TEST: fmt_detail_line formatting and string reuse ───────────────────
    #[test]
    fn fmt_detail_line_correct() {
        let mut buf = String::new();
        let snap = PerfSnapshot {
            crt_us: 1234,
            bloom_us: 567,
            composite_us: 234,
            glass_us: 891,
        };
        fmt_detail_line(&mut buf, &snap);
        assert_eq!(buf, "C:1234 B:567 CS:234 GL:891");

        // Reuses same allocation — clears then writes
        let snap2 = PerfSnapshot::default();
        fmt_detail_line(&mut buf, &snap2);
        assert_eq!(buf, "C:0 B:0 CS:0 GL:0");
    }

    // ── TEST: entering Detailed primes an immediate detail sample ───────────
    #[test]
    fn entering_detailed_primes_detail_sampling() {
        assert!(
            should_prime_detail_sampling(PerfOverlayLevel::Basic, PerfOverlayLevel::Detailed),
            "Basic->Detailed must prime detail sampling so the first Detailed frame is not blank"
        );
        assert!(
            !should_prime_detail_sampling(PerfOverlayLevel::Detailed, PerfOverlayLevel::Off),
            "Detailed->Off must not prime detail sampling"
        );
        assert!(
            !should_prime_detail_sampling(PerfOverlayLevel::Off, PerfOverlayLevel::Basic),
            "Off->Basic must not prime detail sampling"
        );
    }

    // ── TEST: detail row moves below REC/PLAY HUD when active ────────────────
    #[test]
    fn perf_detail_overlay_row_avoids_transport_hud() {
        assert_eq!(
            perf_detail_overlay_y(false),
            SCREEN_Y + 20,
            "without REC/PLAY, detail line stays on the second row"
        );
        assert_eq!(
            perf_detail_overlay_y(true),
            SCREEN_Y + 32,
            "with REC/PLAY, detail line shifts down to avoid overlap"
        );
    }

    // ── TEST: basic row moves below rewind bar when active ────────────────────
    #[test]
    fn perf_basic_overlay_row_avoids_rewind_bar() {
        assert_eq!(
            perf_basic_overlay_y(false),
            SCREEN_Y + 8,
            "without rewind bar, basic FPS line stays on the top row"
        );
        assert_eq!(
            perf_basic_overlay_y(true),
            SCREEN_Y + 20,
            "with rewind bar, basic FPS line shifts down to avoid overlap"
        );
    }

    // ── TEST 5: live-preview regression – scanline_intensity slider ──────────
    // Before the fix, sv_table was only rebuilt on menu-exit (input.back).
    // apply_scanline_intensity_change encapsulates the immediate rebuild that
    // must happen on every slider tick.
    #[test]
    fn sv_table_live_preview_scanline_intensity_change() {
        let vignette = build_vignette_table_with_strength(20);
        let mut sv = build_sv_table(&vignette, 40); // initial state

        // Simulate slider move: 40 → 60.  The fix calls this helper immediately
        // in the slider handler instead of deferring until menu exit.
        apply_scanline_intensity_change(&mut sv, &vignette, 60);

        let expected = build_sv_table(&vignette, 60);
        assert_eq!(
            sv, expected,
            "sv_table must immediately reflect new scanline_intensity for live preview"
        );
    }

    // ── TEST 6: live-preview regression – vignette_strength slider ───────────
    // Same regression: vignette_table (and therefore sv_table) was stale during
    // live preview.  apply_vignette_strength_change must rebuild both at once.
    #[test]
    fn sv_table_live_preview_vignette_strength_change() {
        let mut vignette = build_vignette_table_with_strength(20);
        let scanline_intensity = 40u8;
        let mut sv = build_sv_table(&vignette, scanline_intensity); // initial state

        // Simulate slider move: vignette strength 20 → 50.
        apply_vignette_strength_change(&mut sv, &mut vignette, 50, scanline_intensity);

        let expected_vignette = build_vignette_table_with_strength(50);
        let expected_sv = build_sv_table(&expected_vignette, scanline_intensity);
        assert_eq!(
            sv, expected_sv,
            "sv_table must immediately reflect new vignette_strength for live preview"
        );
        assert_eq!(
            vignette, expected_vignette,
            "vignette_table must immediately reflect new vignette_strength for live preview"
        );
    }
}
