use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{traits::*, HeapRb};
use gilrs::{Gilrs, Button, Axis};
use serde::{Serialize, Deserialize};

use nes_emulator::bus::Bus;
use nes_emulator::cartridge::Cartridge;
use nes_emulator::cpu::Cpu;
use nes_emulator::joypad::JoypadButton;

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
struct EmulatorConfig {
    recent_games: Vec<String>,
    crt_enabled: bool,
    barrel_distortion: bool,
    audio_volume: u32,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            recent_games: Vec::new(),
            crt_enabled: true,
            barrel_distortion: false,
            audio_volume: 100,
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
            if let Ok(cfg) = serde_json::from_str::<EmulatorConfig>(&data) {
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

// =====================================================================
// Save state support (SRAM battery backup emulation)
// =====================================================================

fn save_state_dir() -> PathBuf {
    config_dir().join("saves")
}

fn save_state_path(config: &EmulatorConfig) -> Option<PathBuf> {
    let recent = config.recent_games.first()?;
    let filename = Path::new(recent).file_stem()?.to_string_lossy().to_string();
    Some(save_state_dir().join(format!("{}.sav", filename)))
}

fn save_state(bus: &Bus, _cpu: &Cpu, config: &EmulatorConfig) -> bool {
    let Some(path) = save_state_path(config) else { return false; };
    let _ = fs::create_dir_all(save_state_dir());
    let sram = bus.get_sram();
    if sram.is_empty() { return false; }
    fs::write(&path, &sram).is_ok()
}

fn load_state(bus: &mut Bus, _cpu: &mut Cpu, config: &EmulatorConfig) -> bool {
    let Some(path) = save_state_path(config) else { return false; };
    if !path.exists() { return false; }
    match fs::read(&path) {
        Ok(data) => { bus.set_sram(&data); true }
        Err(_) => false,
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
}

impl MenuState {
    fn new() -> Self {
        Self {
            selected: 0,
            submenu: None,
            cursor_visible: true,
            cursor_timer: 0,
        }
    }
}

enum SubMenu {
    Settings { selected: usize },
    FileBrowser(FileBrowser),
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
    fn new() -> Self {
        let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
        let roms_dir = PathBuf::from(&home).join(".nes-emulator").join("roms");
        let downloads = PathBuf::from(&home).join("Downloads");
        let dir = if roms_dir.is_dir() {
            roms_dir
        } else if downloads.is_dir() {
            downloads
        } else {
            PathBuf::from(".")
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

fn render_home_screen(fb: &mut [u32], menu: &MenuState, cfg: &EmulatorConfig, cursor_visible: bool) {
    for pixel in fb.iter_mut() { *pixel = MENU_BG; }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 NES EMULATOR \x11", 2, MENU_GOLD);
    draw_separator_line(fb, 3);

    let recent_count = cfg.recent_games.len().min(10);
    let browse_idx = recent_count;
    let settings_idx = recent_count + 1;

    let mut current_row: usize = 4;

    if recent_count > 0 {
        draw_text_8x8(fb, "RECENT GAMES", 3, current_row, MENU_DARK_GRAY);
        current_row += 1;

        for i in 0..recent_count {
            let row = current_row + i;
            if row >= 26 { break; }

            let path_str = &cfg.recent_games[i];
            let filename = Path::new(path_str)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.clone());

            let exists = Path::new(path_str).exists();
            let is_selected = i == menu.selected;

            if is_selected {
                for x in 20..236 {
                    let y_base = row * 8;
                    for dy in 0..8 {
                        if y_base + dy < 240 {
                            fb[(y_base + dy) * 256 + x] = 0x3C3C8C;
                        }
                    }
                }
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
        }

        current_row += recent_count + 1;
    } else {
        draw_text_centered_8x8(fb, "NO RECENT GAMES YET", current_row + 1, MENU_DARK_GRAY);
        draw_text_centered_8x8(fb, "BROWSE TO PLAY!", current_row + 2, MENU_DARK_GRAY);
        current_row += 4;
    }

    draw_separator_line(fb, current_row);
    current_row += 1;

    // BROWSE FILES option
    {
        let row = current_row;
        let is_selected = menu.selected == browse_idx;
        if is_selected {
            for x in 20..236 {
                let y_base = row * 8;
                for dy in 0..8 {
                    if y_base + dy < 240 {
                        fb[(y_base + dy) * 256 + x] = 0x3C3C8C;
                    }
                }
            }
        }
        let color = if is_selected { MENU_WHITE } else { MENU_GRAY };
        if is_selected && cursor_visible {
            draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
        }
        draw_text_8x8(fb, "BROWSE FILES", 3, row, color);
        current_row += 1;
    }

    // SETTINGS option
    {
        let row = current_row;
        let is_selected = menu.selected == settings_idx;
        if is_selected {
            for x in 20..236 {
                let y_base = row * 8;
                for dy in 0..8 {
                    if y_base + dy < 240 {
                        fb[(y_base + dy) * 256 + x] = 0x3C3C8C;
                    }
                }
            }
        }
        let color = if is_selected { MENU_WHITE } else { MENU_GRAY };
        if is_selected && cursor_visible {
            draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
        }
        draw_text_8x8(fb, "SETTINGS", 3, row, color);
    }

    draw_separator_line(fb, 25);
    draw_text_centered_8x8(fb, "A:OPEN  ESC:QUIT", 26, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "IN GAME: START+SEL 1s", 27, MENU_DARK_GRAY);
}

fn render_settings(fb: &mut [u32], cfg: &EmulatorConfig, selected: usize, cursor_visible: bool, audio_volume: u32) {
    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 SETTINGS \x11", 4, MENU_GOLD);
    draw_separator_line(fb, 5);

    let settings_items = [
        format!("CRT FILTER: {}", if cfg.crt_enabled { "ON" } else { "OFF" }),
        format!("BARREL DISTORTION: {}", if cfg.barrel_distortion { "ON" } else { "OFF" }),
        format!("AUDIO VOLUME: {}%", audio_volume),
    ];
    let setting_rows = [8, 10, 12];

    for (i, (item, &row)) in settings_items.iter().zip(setting_rows.iter()).enumerate() {
        let color = if i == selected { MENU_WHITE } else { MENU_GRAY };
        if i == selected && cursor_visible {
            draw_char_8x8(fb, '\x10', 3, row, MENU_WHITE);
        }
        draw_text_8x8(fb, item, 5, row, color);
    }

    draw_separator_line(fb, 16);
    draw_text_centered_8x8(fb, "ENTER/LEFT/RIGHT TO CHANGE", 20, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "ESC TO GO BACK", 21, MENU_DARK_GRAY);
}

fn truncate_path_display(path: &Path, max_chars: usize) -> String {
    let s = path.to_string_lossy().to_uppercase();
    if s.len() <= max_chars {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - (max_chars - 3)..])
    }
}

fn render_file_browser(fb: &mut [u32], browser: &FileBrowser, cursor_visible: bool) {
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
            let display_name = if name_upper.len() > 26 {
                format!("{}..", &name_upper[..24])
            } else {
                name_upper
            };

            let display = if entry.is_dir {
                format!("> {}", display_name)
            } else {
                format!("  {}", display_name)
            };

            if is_selected {
                // Highlight bar
                for x in 20..236 {
                    let y_base = row * 8;
                    for dy in 0..8 {
                        if y_base + dy < 240 {
                            fb[(y_base + dy) * 256 + x] = HIGHLIGHT_BG;
                        }
                    }
                }
                if cursor_visible {
                    draw_char_8x8(fb, '\x10', 2, row, MENU_WHITE);
                }
                let color = if entry.is_dir { DIR_COLOR_SEL } else { MENU_WHITE };
                draw_text_8x8(fb, &display, 3, row, color);
                // File size for selected .nes files
                if !entry.is_dir {
                    let size_str = format!("{}K", entry.size_kb);
                    let size_x = 30 - size_str.len();
                    draw_text_8x8(fb, &size_str, size_x, row, MENU_DARK_GRAY);
                }
            } else {
                let color = if entry.is_dir { DIR_COLOR } else { MENU_GRAY };
                draw_text_8x8(fb, &display, 3, row, color);
            }
        }

        // Scroll position indicator
        if browser.entries.len() > VISIBLE_ROWS {
            let pos_str = format!("{}/{}", browser.selected + 1, browser.entries.len());
            let pos_x = 30 - pos_str.len();
            draw_text_8x8(fb, &pos_str, pos_x, 25, MENU_DARK_GRAY);
        }
    }

    draw_separator_line(fb, 25);
    draw_text_centered_8x8(fb, "A:OPEN B:BACK L/R:PAGE", 26, MENU_DARK_GRAY);

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
            if src_x >= 255.99 || src_y >= 239.99 {
                table.push((0xFFFFFFFF, 0));
            } else {
                let src_xf = (src_x * 256.0) as u32;
                let src_yf = (src_y * 256.0) as u32;
                table.push((src_xf, src_yf));
            }
        }
    }
    table
}

// =====================================================================
// Menu input handling
// =====================================================================

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
            *counter == 1 || (*counter > 18 && (*counter - 18) % 4 == 0)
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
}

fn poll_menu_input(window: &Window, gilrs: &mut Option<Gilrs>, repeat: &mut RepeatTracker) -> MenuInput {
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
    };

    if let Some(ref mut g) = gilrs {
        // Event-driven for one-shot buttons
        while let Some(event) = g.next_event() {
            if let gilrs::EventType::ButtonPressed(btn, _) = event.event {
                match btn {
                    Button::Start | Button::South => mi.confirm = true,
                    Button::East => mi.back = true,
                    Button::LeftTrigger | Button::LeftTrigger2 => mi.page_up = true,
                    Button::RightTrigger | Button::RightTrigger2 => mi.page_down = true,
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
            let deadzone = 0.3;
            if stick_y > deadzone { raw_up = true; }
            if stick_y < -deadzone { raw_down = true; }
            if stick_x < -deadzone { raw_left = true; }
            if stick_x > deadzone { raw_right = true; }
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
}

fn play_menu_sound<P: ringbuf::traits::Producer<Item = f32>>(producer: &mut P, sound: MenuSound, sample_rate: u32, volume: f32) {
    let vol = volume.max(0.3) * 0.15; // minimum 30% so menu is always audible
    match sound {
        MenuSound::Cursor => generate_menu_tone(producer, 880.0, 30, vol, sample_rate),
        MenuSound::Confirm => generate_menu_tone(producer, 440.0, 60, vol, sample_rate),
        MenuSound::Back => generate_menu_tone(producer, 330.0, 40, vol, sample_rate),
        MenuSound::Error => generate_menu_tone(producer, 220.0, 100, vol, sample_rate),
    }
}

// =====================================================================
// Main function
// =====================================================================

fn main() {
    let mut config = load_config();

    let mut window = Window::new(
        "NES Emulator",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions {
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create window");

    window.set_target_fps(60);

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

    // Build static TV frame once at startup (zero per-frame cost)
    let mut tv_frame_bg = Vec::new();
    build_tv_frame(&mut tv_frame_bg);
    build_console_overlay(&mut tv_frame_bg, TV_HEIGHT, WINDOW_WIDTH, WINDOW_HEIGHT);
    let mut composite_buffer = vec![0u32; WINDOW_WIDTH * WINDOW_HEIGHT];

    // Pre-compute vignette lookup table (same every frame)
    let vignette_table = {
        let mut table = vec![0u16; SCREEN_W * SCREEN_H];
        for y in 0..SCREEN_H {
            for x in 0..SCREEN_W {
                let fx = (x as f32 / SCREEN_W as f32) - 0.5;
                let fy = (y as f32 / SCREEN_H as f32) - 0.5;
                let v = 1.0 - (fx * fx + fy * fy) * 1.5;
                table[y * SCREEN_W + x] = (v.max(0.3).min(1.0) * 256.0) as u16;
            }
        }
        table
    };
    // Pre-compute barrel distortion lookup table for curved CRT glass
    let distortion_table: Vec<(u32, u32)> = {
        let mut table = Vec::with_capacity(SCREEN_W * SCREEN_H);
        for dst_y in 0..SCREEN_H {
            for dst_x in 0..SCREEN_W {
                let nx = (dst_x as f32 / SCREEN_W as f32) * 2.0 - 1.0;
                let ny = (dst_y as f32 / SCREEN_H as f32) * 2.0 - 1.0;
                let r2 = nx * nx + ny * ny;
                let distortion = 1.0 + 0.015 * r2;
                let dx = nx / distortion;
                let dy = ny / distortion;
                let src_x = ((dx + 1.0) / 2.0) * 256.0;
                let src_y = ((dy + 1.0) / 2.0) * 240.0;
                if src_x < 0.0 || src_x >= 255.99 || src_y < 0.0 || src_y >= 239.99 {
                    table.push((0xFFFFFFFF, 0));
                } else {
                    let src_xf = (src_x * 256.0) as u32;
                    let src_yf = (src_y * 256.0) as u32;
                    table.push((src_xf, src_yf));
                }
            }
        }
        table
    };
    let flat_distortion_table = build_flat_distortion_table();
    let glare_table = build_glare_table();
    let mut crt_enabled = config.crt_enabled;
    let mut barrel_distortion = config.barrel_distortion;
    let mut audio_volume = config.audio_volume;
    let mut mouse_was_down = false;

    // Menu framebuffer (256x240, same as NES PPU output)
    let mut menu_framebuffer = vec![0u32; 256 * 240];

    // State machine
    let mut emulator_state = EmulatorState::Menu(MenuState::new());
    let mut game_bus: Option<Bus> = None;
    let mut game_cpu: Option<Cpu> = None;
    let mut frame_counter: u32 = 0;
    let mut quit_hold_frames: u32 = 0;
    let mut repeat_tracker = RepeatTracker::new();
    let mut overlay_message: Option<String> = None;
    let mut overlay_timer: u32 = 0;
    let mut sound_cooldown: u32 = 0;
    
    // Pause menu state
    let mut paused: bool = false;
    let mut pause_selected: usize = 0;

    // Check command-line argument for direct ROM load
    let args: Vec<String> = env::args().collect();
    if let Some(rom_path) = args.get(1) {
        if let Ok(rom_data) = fs::read(rom_path) {
            if let Ok(cart) = Cartridge::new(&rom_data) {
                let mut bus = Bus::new(cart);
                bus.set_apu_sample_rate(actual_sample_rate);
                let mut cpu = Cpu::new();
                cpu.reset(&mut bus);
                add_recent_game(&mut config, rom_path);
                save_config(&config);
                game_bus = Some(bus);
                game_cpu = Some(cpu);
                emulator_state = EmulatorState::Game;
                println!("Loaded: {}", rom_path);
            }
        }
    }

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

                let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker);

                let mut action: Option<MenuAction> = None;

                match menu.submenu {
                    None => {
                        let recent_count = config.recent_games.len().min(10);
                        let browse_idx = recent_count;
                        let settings_idx = recent_count + 1;
                        let total_items = recent_count + 2;

                        if input.up && menu.selected > 0 {
                            menu.selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3; // skip 3 frames between beeps
                            }
                        }
                        if input.down && menu.selected < total_items - 1 {
                            menu.selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3; // skip 3 frames between beeps
                            }
                        }
                        if input.confirm {
                            if menu.selected < recent_count {
                                let path = config.recent_games[menu.selected].clone();
                                if Path::new(&path).exists() {
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                    action = Some(MenuAction::LoadRom(path));
                                } else {
                                    play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                            } else if menu.selected == browse_idx {
                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                menu.submenu = Some(SubMenu::FileBrowser(FileBrowser::new()));
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                            } else if menu.selected == settings_idx {
                                play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                menu.submenu = Some(SubMenu::Settings { selected: 0 });
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                            }
                        }
                        if input.back {
                            break;
                        }
                    }
                    Some(SubMenu::Settings { ref mut selected }) => {
                        if input.up && *selected > 0 {
                            *selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3; // skip 3 frames between beeps
                            }
                        }
                        if input.down && *selected < 2 {
                            *selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3; // skip 3 frames between beeps
                            }
                        }
                        if input.confirm || input.left || input.right {
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
                                        if audio_volume < 100 {
                                            audio_volume = (audio_volume + 10).min(100);
                                        }
                                    }
                                    if input.left {
                                        audio_volume = audio_volume.saturating_sub(10);
                                    }
                                    config.audio_volume = audio_volume;
                                    save_config(&config);
                                }
                                _ => {}
                            }
                            play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                        }
                        if input.back {
                            play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                            menu.submenu = None;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
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
                            if input.page_up && browser.selected > 0 {
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
                            if input.page_down && count > 0 {
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
                                }
                            } else {
                                menu.submenu = None;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                            }
                        }
                    }
                }

                // Process actions
                match action {
                    Some(MenuAction::LoadRom(path_str)) => {
                        match fs::read(&path_str) {
                            Ok(rom_data) => {
                                match Cartridge::new(&rom_data) {
                                    Ok(cart) => {
                                        let mut bus = Bus::new(cart);
                                        bus.set_apu_sample_rate(actual_sample_rate);
                                        let mut cpu = Cpu::new();
                                        cpu.reset(&mut bus);
                                        add_recent_game(&mut config, &path_str);
                                        save_config(&config);
                                        game_bus = Some(bus);
                                        game_cpu = Some(cpu);
                                        next_state = Some(EmulatorState::Game);
                                        println!("Loaded: {}", path_str);
                                    }
                                    Err(e) => {
                                        play_menu_sound(&mut producer, MenuSound::Error, actual_sample_rate, audio_volume as f32 / 100.0);
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
                    None => render_home_screen(&mut menu_framebuffer, menu, &config, menu.cursor_visible),
                    Some(SubMenu::Settings { selected }) => {
                        render_settings(&mut menu_framebuffer, &config, selected, menu.cursor_visible, audio_volume);
                    }
                    Some(SubMenu::FileBrowser(ref browser)) => {
                        render_file_browser(&mut menu_framebuffer, browser, menu.cursor_visible);
                    }
                }

                // Apply CRT filter pipeline (same as game!)
                let dt = if barrel_distortion { &distortion_table } else { &flat_distortion_table };
                if crt_enabled {
                    crt_filter(&menu_framebuffer, &mut crt_buffer, &vignette_table, dt);
                } else {
                    scale_simple(&menu_framebuffer, &mut crt_buffer);
                }
                composite_screen(&tv_frame_bg, &crt_buffer, &mut composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT);
                if crt_enabled {
                    apply_screen_glare(&mut composite_buffer, &glare_table, WINDOW_WIDTH);
                }

                window
                    .update_with_buffer(&composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
                    .expect("Failed to update window");
            }

            EmulatorState::Game => {
                if let (Some(ref mut bus), Some(ref mut cpu)) = (&mut game_bus, &mut game_cpu) {
                    // Handle pause menu input
                    if paused {
                        let input = poll_menu_input(&window, &mut gilrs, &mut repeat_tracker);
                        if input.up && pause_selected > 0 {
                            pause_selected -= 1;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }
                        if input.down && pause_selected < 3 {
                            pause_selected += 1;
                            if sound_cooldown == 0 {
                                play_menu_sound(&mut producer, MenuSound::Cursor, actual_sample_rate, audio_volume as f32 / 100.0);
                                sound_cooldown = 3;
                            }
                        }
                        if input.confirm {
                            match pause_selected {
                                0 => { // Resume
                                    paused = false;
                                    play_menu_sound(&mut producer, MenuSound::Confirm, actual_sample_rate, audio_volume as f32 / 100.0);
                                }
                                1 => { // Save state
                                    if save_state(bus, cpu, &config) {
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
                                    if load_state(bus, cpu, &config) {
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
                                3 => { // Return to menu
                                    game_bus = None;
                                    game_cpu = None;
                                    paused = false;
                                    quit_hold_frames = 0;
                                    emulator_state = EmulatorState::Menu(MenuState::new());
                                    play_menu_sound(&mut producer, MenuSound::Back, actual_sample_rate, audio_volume as f32 / 100.0);
                                    continue;
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

                        // End APU frame and get band-limited samples
                        bus.apu.end_frame();

                        // Push audio samples with volume control
                        {
                            let samples = bus.apu.drain_samples();
                            let vol = audio_volume as f32 / 100.0;
                            for &sample in &samples {
                                let _ = producer.try_push(sample * vol);
                            }
                        }
                        
                        // Handle input when not paused
                        frame_counter = frame_counter.wrapping_add(1);
                        let (start_held, select_held) = handle_input(&window, bus, &mut gilrs, frame_counter);

                        // Gamepad quit combo: hold Start+Select for ~1 second (60 frames)
                        if start_held && select_held {
                            quit_hold_frames += 1;
                            if quit_hold_frames >= 60 {
                                game_bus = None;
                                game_cpu = None;
                                quit_hold_frames = 0;
                                repeat_tracker = RepeatTracker::new();
                                emulator_state = EmulatorState::Menu(MenuState::new());
                                continue;
                            }
                        } else {
                            quit_hold_frames = 0;
                        }

                        // F5 quick save, F9 quick load
                        if window.is_key_pressed(Key::F5, KeyRepeat::No) {
                            if save_state(bus, cpu, &config) {
                                overlay_message = Some("STATE SAVED".to_string());
                                overlay_timer = 90;
                            } else {
                                overlay_message = Some("NO SRAM FOUND".to_string());
                                overlay_timer = 90;
                            }
                        }
                        if window.is_key_pressed(Key::F9, KeyRepeat::No) {
                            if load_state(bus, cpu, &config) {
                                overlay_message = Some("STATE LOADED".to_string());
                                overlay_timer = 90;
                            } else {
                                overlay_message = Some("NO SAVE FOUND".to_string());
                                overlay_timer = 90;
                            }
                        }

                        if window.is_key_pressed(Key::F1, KeyRepeat::No) {
                            crt_enabled = !crt_enabled;
                            config.crt_enabled = crt_enabled;
                            save_config(&config);
                            overlay_message = Some(if crt_enabled { "CRT FILTER: ON".to_string() } else { "CRT FILTER: OFF".to_string() });
                            overlay_timer = 90; // 1.5 seconds
                        }

                        // Escape toggles pause menu
                        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                            paused = true;
                            pause_selected = 0;
                        }
                    }

                    // ALWAYS render (even when paused - shows frozen frame)
                    let dt = if barrel_distortion { &distortion_table } else { &flat_distortion_table };
                    if crt_enabled {
                        crt_filter(&bus.ppu.frame_data, &mut crt_buffer, &vignette_table, dt);
                    } else {
                        scale_simple(&bus.ppu.frame_data, &mut crt_buffer);
                    }

                    // Composite game output into TV frame
                    composite_screen(&tv_frame_bg, &crt_buffer, &mut composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT);

                    if crt_enabled {
                        apply_screen_glare(&mut composite_buffer, &glare_table, WINDOW_WIDTH);
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
                        // Copy and darken the last game frame into menu_framebuffer
                        for i in 0..menu_framebuffer.len().min(bus.ppu.frame_data.len()) {
                            let p = bus.ppu.frame_data[i];
                            let r = ((p >> 16) & 0xFF) / 3;
                            let g = ((p >> 8) & 0xFF) / 3;
                            let b = (p & 0xFF) / 3;
                            menu_framebuffer[i] = (r << 16) | (g << 8) | b;
                        }
                        
                        // Box background (tile coordinates: 32 cols × 30 rows)
                        // Center a box roughly 20 tiles wide × 14 tiles tall
                        let box_left = 6;
                        let box_right = 26;
                        let box_top = 8;
                        let box_bottom = 22;
                        
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
                        
                        // Title: "PAUSED" centered
                        draw_text_centered_8x8(&mut menu_framebuffer, "\x11 PAUSED \x11", box_top + 1, MENU_GOLD);
                        
                        // Separator
                        let sep_y = (box_top + 2) * 8 + 4;
                        for x in (box_left * 8 + 8)..(box_right * 8 - 8) {
                            if x % 4 < 2 && sep_y < 240 {
                                menu_framebuffer[sep_y * 256 + x] = MENU_DARK_GRAY;
                            }
                        }
                        
                        // Menu items
                        let items = ["RESUME GAME", "SAVE STATE  (F5)", "LOAD STATE  (F9)", "RETURN TO MENU"];
                        for (i, item) in items.iter().enumerate() {
                            let row = box_top + 4 + i * 2; // rows 12, 14, 16, 18
                            let is_selected = i == pause_selected;
                            
                            if is_selected {
                                // Highlight bar
                                let hy = row * 8;
                                for dy in 0..8 {
                                    for hx in (box_left * 8 + 4)..(box_right * 8 - 4) {
                                        if hy + dy < 240 && hx < 256 {
                                            menu_framebuffer[(hy + dy) * 256 + hx] = 0x3C3C8C;
                                        }
                                    }
                                }
                                draw_char_8x8(&mut menu_framebuffer, '\x10', box_left + 1, row, MENU_WHITE);
                            }
                            
                            let color = if is_selected { MENU_WHITE } else { MENU_GRAY };
                            draw_text_8x8(&mut menu_framebuffer, item, box_left + 2, row, color);
                        }
                        
                        // Hint at bottom of box
                        draw_text_centered_8x8(&mut menu_framebuffer, "ESC:RESUME  A:SELECT", box_bottom - 1, MENU_DARK_GRAY);
                        
                        // Now pass through CRT filter (same as menu rendering)
                        let dt = if barrel_distortion { &distortion_table } else { &flat_distortion_table };
                        if crt_enabled {
                            crt_filter(&menu_framebuffer, &mut crt_buffer, &vignette_table, dt);
                        } else {
                            scale_simple(&menu_framebuffer, &mut crt_buffer);
                        }
                        composite_screen(&tv_frame_bg, &crt_buffer, &mut composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT);
                        if crt_enabled {
                            apply_screen_glare(&mut composite_buffer, &glare_table, WINDOW_WIDTH);
                        }
                    }

                    window
                        .update_with_buffer(&composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
                        .expect("Failed to update window");

                    // Mouse click handling for console interactions (only when not paused)
                    if !paused {
                        let mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
                        mouse_was_down = mouse_down;
                    }
                } else {
                    next_state = Some(EmulatorState::Menu(MenuState::new()));
                }
            }
        }

        // Apply deferred state transitions
        if let Some(new_state) = next_state {
            emulator_state = new_state;
        }
    }
}


fn handle_input(window: &Window, bus: &mut Bus, gilrs: &mut Option<Gilrs>, frame_counter: u32) -> (bool, bool) {
    let keys = window.get_keys();
    let turbo_active = (frame_counter / 2) % 2 == 0; // ~15Hz: ON 2 frames, OFF 2 frames

    // Keyboard: regular buttons
    let mut a_pressed = keys.contains(&Key::A);
    let mut b_pressed = keys.contains(&Key::S);
    let mut select_pressed = keys.contains(&Key::RightShift);
    let mut start_pressed = keys.contains(&Key::Enter);
    let mut up_pressed = keys.contains(&Key::Up);
    let mut down_pressed = keys.contains(&Key::Down);
    let mut left_pressed = keys.contains(&Key::Left);
    let mut right_pressed = keys.contains(&Key::Right);

    // Keyboard: turbo buttons (Z = Turbo A, X = Turbo B)
    if keys.contains(&Key::Z) && turbo_active {
        a_pressed = true;
    }
    if keys.contains(&Key::X) && turbo_active {
        b_pressed = true;
    }

    // Poll gamepad events and read state
    if let Some(ref mut g) = gilrs {
        // Process pending events (required by gilrs)
        while let Some(_event) = g.next_event() {}

        // Read first connected gamepad
        if let Some((_id, gamepad)) = g.gamepads().find(|(_, gp)| gp.is_connected()) {
            // D-pad buttons
            up_pressed |= gamepad.is_pressed(Button::DPadUp);
            down_pressed |= gamepad.is_pressed(Button::DPadDown);
            left_pressed |= gamepad.is_pressed(Button::DPadLeft);
            right_pressed |= gamepad.is_pressed(Button::DPadRight);

            // Left analog stick (with deadzone)
            let stick_x = gamepad.value(Axis::LeftStickX);
            let stick_y = gamepad.value(Axis::LeftStickY);
            let deadzone = 0.3;

            if stick_x < -deadzone { left_pressed = true; }
            if stick_x > deadzone { right_pressed = true; }
            if stick_y > deadzone { up_pressed = true; }
            if stick_y < -deadzone { down_pressed = true; }

            // Face buttons
            // Regular: Xbox A (South) → NES A, Xbox X (West) → NES B
            a_pressed |= gamepad.is_pressed(Button::South);
            b_pressed |= gamepad.is_pressed(Button::West);

            // Turbo: Xbox B (East) → Turbo NES A, Xbox Y (North) → Turbo NES B
            if gamepad.is_pressed(Button::East) && turbo_active {
                a_pressed = true;
            }
            if gamepad.is_pressed(Button::North) && turbo_active {
                b_pressed = true;
            }

            // Start / Select
            start_pressed |= gamepad.is_pressed(Button::Start);
            select_pressed |= gamepad.is_pressed(Button::Select);
            select_pressed |= gamepad.is_pressed(Button::Mode);
        }
    }

    // Apply all input to joypad
    bus.joypad1.set_button_pressed(JoypadButton::A, a_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::B, b_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Select, select_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Start, start_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Up, up_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Down, down_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Left, left_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Right, right_pressed);

    // Player 2 - second gamepad
    let mut p2_a = false;
    let mut p2_b = false;
    let mut p2_select = false;
    let mut p2_start = false;
    let mut p2_up = false;
    let mut p2_down = false;
    let mut p2_left = false;
    let mut p2_right = false;

    if let Some(ref mut g) = gilrs {
        // Find second connected gamepad
        let gamepads: Vec<_> = g.gamepads().filter(|(_, gp)| gp.is_connected()).collect();
        if gamepads.len() >= 2 {
            let (_, gamepad) = &gamepads[1];
            p2_up |= gamepad.is_pressed(Button::DPadUp);
            p2_down |= gamepad.is_pressed(Button::DPadDown);
            p2_left |= gamepad.is_pressed(Button::DPadLeft);
            p2_right |= gamepad.is_pressed(Button::DPadRight);
            
            let stick_x = gamepad.value(Axis::LeftStickX);
            let stick_y = gamepad.value(Axis::LeftStickY);
            let deadzone = 0.3;
            if stick_x < -deadzone { p2_left = true; }
            if stick_x > deadzone { p2_right = true; }
            if stick_y > deadzone { p2_up = true; }
            if stick_y < -deadzone { p2_down = true; }
            
            p2_a |= gamepad.is_pressed(Button::South);
            p2_b |= gamepad.is_pressed(Button::West);
            if gamepad.is_pressed(Button::East) && turbo_active { p2_a = true; }
            if gamepad.is_pressed(Button::North) && turbo_active { p2_b = true; }
            p2_start |= gamepad.is_pressed(Button::Start);
            p2_select |= gamepad.is_pressed(Button::Select);
        }
    }

    bus.joypad2.set_button_pressed(JoypadButton::A, p2_a);
    bus.joypad2.set_button_pressed(JoypadButton::B, p2_b);
    bus.joypad2.set_button_pressed(JoypadButton::Select, p2_select);
    bus.joypad2.set_button_pressed(JoypadButton::Start, p2_start);
    bus.joypad2.set_button_pressed(JoypadButton::Up, p2_up);
    bus.joypad2.set_button_pressed(JoypadButton::Down, p2_down);
    bus.joypad2.set_button_pressed(JoypadButton::Left, p2_left);
    bus.joypad2.set_button_pressed(JoypadButton::Right, p2_right);

    (start_pressed, select_pressed)
}

fn crt_filter(input: &[u32], output: &mut Vec<u32>, vignette_table: &[u16], distortion_table: &[(u32, u32)]) {
    output.resize(SCREEN_W * SCREEN_H, 0);
    
    for dst_y in 0..SCREEN_H {
        let scan_mul: u32 = match dst_y % 3 {
            0 => 255,
            1 => 245,
            2 => 195,
            _ => 255,
        };
        
        let dst_row = dst_y * SCREEN_W;
        
        for dst_x in 0..SCREEN_W {
            let table_idx = dst_y * SCREEN_W + dst_x;
            let (src_xf, src_yf) = distortion_table[table_idx];
            
            // Out-of-bounds pixels (barrel distortion edges) → black
            if src_xf == 0xFFFFFFFF {
                output[dst_row + dst_x] = 0x000000;
                continue;
            }
            
            let src_x0 = (src_xf >> 8) as usize;
            let src_x1 = (src_x0 + 1).min(255);
            let frac_x = (src_xf & 0xFF) as u32;
            
            let src_y0 = (src_yf >> 8) as usize;
            let src_y1 = (src_y0 + 1).min(239);
            let frac_y = (src_yf & 0xFF) as u32;
            
            // Bilinear interpolation
            let p00 = input[src_y0 * 256 + src_x0];
            let p10 = input[src_y0 * 256 + src_x1];
            let p01 = input[src_y1 * 256 + src_x0];
            let p11 = input[src_y1 * 256 + src_x1];
            
            let inv_fx = 256 - frac_x;
            let inv_fy = 256 - frac_y;
            
            let r00 = (p00 >> 16) & 0xFF; let r10 = (p10 >> 16) & 0xFF;
            let r01 = (p01 >> 16) & 0xFF; let r11 = (p11 >> 16) & 0xFF;
            let mut r = (r00 * inv_fx * inv_fy + r10 * frac_x * inv_fy 
                       + r01 * inv_fx * frac_y + r11 * frac_x * frac_y) >> 16;
            
            let g00 = (p00 >> 8) & 0xFF; let g10 = (p10 >> 8) & 0xFF;
            let g01 = (p01 >> 8) & 0xFF; let g11 = (p11 >> 8) & 0xFF;
            let mut g = (g00 * inv_fx * inv_fy + g10 * frac_x * inv_fy 
                       + g01 * inv_fx * frac_y + g11 * frac_x * frac_y) >> 16;
            
            let b00 = p00 & 0xFF; let b10 = p10 & 0xFF;
            let b01 = p01 & 0xFF; let b11 = p11 & 0xFF;
            let mut b = (b00 * inv_fx * inv_fy + b10 * frac_x * inv_fy 
                       + b01 * inv_fx * frac_y + b11 * frac_x * frac_y) >> 16;
            
            // Horizontal blur
            if src_x0 > 0 && src_x0 < 255 {
                let left = input[src_y0 * 256 + src_x0 - 1];
                let right = input[src_y0 * 256 + src_x1.min(255)];
                let lr = (left >> 16) & 0xFF; let rr = (right >> 16) & 0xFF;
                let lg = (left >> 8) & 0xFF;  let rg = (right >> 8) & 0xFF;
                let lb = left & 0xFF;          let rb = right & 0xFF;
                r = (r * 205 + lr * 25 + rr * 25) >> 8;
                g = (g * 205 + lg * 25 + rg * 25) >> 8;
                b = (b * 205 + lb * 25 + rb * 25) >> 8;
            }
            
            // Brightness boost
            r = (r * 275) >> 8;
            g = (g * 275) >> 8;
            b = (b * 275) >> 8;
            
            // Warm color temperature
            r = (r * 262) >> 8;
            g = (g * 256) >> 8;
            b = (b * 242) >> 8;
            
            // Scanline
            r = (r * scan_mul) >> 8;
            g = (g * scan_mul) >> 8;
            b = (b * scan_mul) >> 8;
            
            // Vignette
            let vig = vignette_table[dst_y * SCREEN_W + dst_x] as u32;
            r = (r * vig) >> 8;
            g = (g * vig) >> 8;
            b = (b * vig) >> 8;
            
            output[dst_row + dst_x] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
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

fn sq_dist(x1: usize, y1: usize, x2: usize, y2: usize) -> usize {
    let dx = if x1 > x2 { x1 - x2 } else { x2 - x1 };
    let dy = if y1 > y2 { y1 - y2 } else { y2 - y1 };
    dx * dx + dy * dy
}

fn composite_screen(tv_frame: &[u32], game_output: &[u32], result: &mut Vec<u32>, window_width: usize, window_height: usize) {
    result.resize(window_width * window_height, 0);
    result.copy_from_slice(tv_frame);
    let corner_r = 12usize;
    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            let in_corner =
                (x < corner_r && y < corner_r && sq_dist(x, y, corner_r, corner_r) > corner_r * corner_r)
                || (x >= SCREEN_W - corner_r && y < corner_r && sq_dist(x, y, SCREEN_W - 1 - corner_r, corner_r) > corner_r * corner_r)
                || (x < corner_r && y >= SCREEN_H - corner_r && sq_dist(x, y, corner_r, SCREEN_H - 1 - corner_r) > corner_r * corner_r)
                || (x >= SCREEN_W - corner_r && y >= SCREEN_H - corner_r && sq_dist(x, y, SCREEN_W - 1 - corner_r, SCREEN_H - 1 - corner_r) > corner_r * corner_r);
            if in_corner { continue; }
            let dst = (y + SCREEN_Y) * window_width + (x + SCREEN_X);
            result[dst] = game_output[y * SCREEN_W + x];
        }
    }
}

fn build_glare_table() -> Vec<u8> {

    let mut table = vec![0u8; SCREEN_W * SCREEN_H];

    // Diagonal glare band: line from (0, 0.1*H) to (W, 0.7*H)
    let a = 0.6 * SCREEN_H as f32;
    let b = -(SCREEN_W as f32);
    let c = 0.1 * SCREEN_H as f32 * SCREEN_W as f32;
    let norm = (a * a + b * b).sqrt();
    let band_sigma = 180.0_f32;
    let band_peak = 35.0_f32; // curved glass catches more light

    // Specular highlight: small bright spot upper-left
    let spec_x = 150.0_f32;
    let spec_y = 120.0_f32;
    let spec_sigma_sq = 35.0_f32 * 35.0;
    let spec_peak = 60.0_f32; // curved glass specular highlight

    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            let fx = x as f32;
            let fy = y as f32;

            // Perpendicular distance to diagonal line
            let dist = (a * fx + b * fy + c).abs() / norm;
            let band = band_peak * (-dist * dist / (2.0 * band_sigma * band_sigma)).exp();

            // Fade: strongest upper-left, fades toward lower-right
            let fade = 1.0 - 0.5 * (fx / SCREEN_W as f32 + fy / SCREEN_H as f32);
            let band = band * fade.max(0.0);

            // Specular highlight (radial Gaussian)
            let dx = fx - spec_x;
            let dy = fy - spec_y;
            let spec = spec_peak * (-(dx * dx + dy * dy) / (2.0 * spec_sigma_sq)).exp();

            table[y * SCREEN_W + x] = (band + spec).min(30.0) as u8;
        }
    }
    table
}

fn apply_screen_glare(buffer: &mut [u32], glare_table: &[u8], window_width: usize) {
    let corner_r = 12usize;
    for y in 0..SCREEN_H {
        let buf_row = (y + SCREEN_Y) * window_width + SCREEN_X;
        let glare_row = y * SCREEN_W;
        for x in 0..SCREEN_W {
            let in_corner =
                (x < corner_r && y < corner_r && sq_dist(x, y, corner_r, corner_r) > corner_r * corner_r)
                || (x >= SCREEN_W - corner_r && y < corner_r && sq_dist(x, y, SCREEN_W - 1 - corner_r, corner_r) > corner_r * corner_r)
                || (x < corner_r && y >= SCREEN_H - corner_r && sq_dist(x, y, corner_r, SCREEN_H - 1 - corner_r) > corner_r * corner_r)
                || (x >= SCREEN_W - corner_r && y >= SCREEN_H - corner_r && sq_dist(x, y, SCREEN_W - 1 - corner_r, SCREEN_H - 1 - corner_r) > corner_r * corner_r);
            if in_corner { continue; }

            let glare = glare_table[glare_row + x] as u32;
            if glare == 0 { continue; }

            let pixel = buffer[buf_row + x];
            let r = ((pixel >> 16) & 0xFF) + glare;
            let g = ((pixel >> 8) & 0xFF) + glare;
            let b = (pixel & 0xFF) + glare;

            buffer[buf_row + x] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
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
