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
    RecentGames { selected: usize },
    Settings { selected: usize },
    FileBrowser(FileBrowser),
}

struct FileBrowserEntry {
    name: String,
    is_dir: bool,
    full_path: PathBuf,
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
        let downloads = PathBuf::from(
            env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string()),
        )
        .join("Downloads");
        let dir = if downloads.is_dir() {
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
                self.error_timer = 120;
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
            });
        } else if name.to_lowercase().ends_with(".nes") {
            files.push(FileBrowserEntry {
                name,
                is_dir: false,
                full_path: path,
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

fn render_main_menu(fb: &mut [u32], menu: &MenuState) {
    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    // Title
    draw_text_centered_8x8(fb, "\x11 NES EMULATOR \x11", 4, MENU_GOLD);
    draw_separator_line(fb, 5);

    // Menu options
    let options = ["LOAD CARTRIDGE", "RECENT GAMES", "SETTINGS"];
    let option_rows = [8, 10, 12];

    for (i, (opt, &row)) in options.iter().zip(option_rows.iter()).enumerate() {
        let color = if i == menu.selected { MENU_WHITE } else { MENU_GRAY };
        if i == menu.selected && menu.cursor_visible {
            draw_char_8x8(fb, '\x10', 4, row, MENU_WHITE);
        }
        draw_text_8x8(fb, opt, 6, row, color);
    }

    draw_separator_line(fb, 16);

    draw_text_centered_8x8(fb, "USE UP/DOWN TO SELECT", 20, MENU_DARK_GRAY);
    draw_text_centered_8x8(fb, "PRESS START TO CONFIRM", 21, MENU_DARK_GRAY);
}

fn render_recent_games(fb: &mut [u32], cfg: &EmulatorConfig, selected: usize, cursor_visible: bool) {
    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 RECENT GAMES \x11", 4, MENU_GOLD);
    draw_separator_line(fb, 5);

    if cfg.recent_games.is_empty() {
        draw_text_centered_8x8(fb, "NO RECENT GAMES", 12, MENU_DARK_GRAY);
    } else {
        for (i, path_str) in cfg.recent_games.iter().enumerate() {
            if i >= 10 { break; }
            let row = 7 + i * 2;
            if row >= 27 { break; }
            let filename = Path::new(path_str)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.clone());

            let exists = Path::new(path_str).exists();
            let color = if !exists {
                MENU_DARK_GRAY
            } else if i == selected {
                MENU_WHITE
            } else {
                MENU_GRAY
            };

            if i == selected && cursor_visible {
                draw_char_8x8(fb, '\x10', 3, row, MENU_WHITE);
            }

            let display: String = filename.chars().take(25).collect();
            draw_text_8x8(fb, &display, 5, row, color);
        }
    }

    draw_separator_line(fb, 27);
    draw_text_centered_8x8(fb, "ESC TO GO BACK", 29, MENU_DARK_GRAY);
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
    const HIGHLIGHT_BG: u32 = 0x2C2C6C;

    for pixel in fb.iter_mut() {
        *pixel = MENU_BG;
    }

    draw_double_border_top(fb, 1);
    draw_double_border_bottom(fb, 28);
    draw_side_borders(fb);

    draw_text_centered_8x8(fb, "\x11 LOAD CARTRIDGE \x11", 2, MENU_GOLD);

    let path_str = truncate_path_display(&browser.current_dir, 28);
    draw_text_8x8(fb, &path_str, 3, 3, MENU_DARK_GRAY);

    draw_separator_line(fb, 4);

    if browser.entries.is_empty() {
        draw_text_centered_8x8(fb, "NO FILES FOUND", 14, MENU_DARK_GRAY);
    } else {
        let start = browser.scroll_offset;
        let end = (start + VISIBLE_ROWS).min(browser.entries.len());

        // Scroll indicators
        if start > 0 {
            draw_text_8x8(fb, "...", 28, FIRST_ROW, MENU_DARK_GRAY);
        }
        if end < browser.entries.len() {
            draw_text_8x8(fb, "...", 28, FIRST_ROW + VISIBLE_ROWS - 1, MENU_DARK_GRAY);
        }

        for i in start..end {
            let row = FIRST_ROW + (i - start);
            let entry = &browser.entries[i];
            let is_selected = i == browser.selected;

            let name_upper = entry.name.to_uppercase();
            let display_name = if name_upper.len() > 20 {
                format!("{}..", &name_upper[..18])
            } else {
                name_upper
            };

            let display = if entry.is_dir {
                format!("[DIR] {}", display_name)
            } else {
                format!("      {}", display_name)
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
            } else {
                let color = if entry.is_dir { DIR_COLOR } else { MENU_GRAY };
                draw_text_8x8(fb, &display, 3, row, color);
            }
        }
    }

    draw_separator_line(fb, 25);
    draw_text_centered_8x8(fb, "UP/DN:SELECT A:OPEN B:BACK", 26, MENU_DARK_GRAY);

    // Error overlay
    if let Some(ref msg) = browser.error_message {
        let msg_upper = msg.to_uppercase();
        let box_row = 13;
        for x in 40..216 {
            for dy in 0..24 {
                let y = box_row * 8 + dy;
                if y < 240 {
                    fb[y * 256 + x] = 0x600000;
                }
            }
        }
        draw_text_centered_8x8(fb, &msg_upper, box_row + 1, 0xFC4444);
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

struct MenuInput {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    confirm: bool,
    back: bool,
    backspace: bool,
}

fn poll_menu_input(window: &Window, gilrs: &mut Option<Gilrs>) -> MenuInput {
    let up = window.is_key_pressed(Key::Up, KeyRepeat::No);
    let down = window.is_key_pressed(Key::Down, KeyRepeat::No);
    let left = window.is_key_pressed(Key::Left, KeyRepeat::No);
    let right = window.is_key_pressed(Key::Right, KeyRepeat::No);
    let confirm = window.is_key_pressed(Key::Enter, KeyRepeat::No);
    let back = window.is_key_pressed(Key::Escape, KeyRepeat::No);
    let backspace = window.is_key_pressed(Key::Backspace, KeyRepeat::No);

    let mut mi = MenuInput { up, down, left, right, confirm, back, backspace };

    if let Some(ref mut g) = gilrs {
        while let Some(event) = g.next_event() {
            if let gilrs::EventType::ButtonPressed(btn, _) = event.event {
                match btn {
                    Button::DPadUp => mi.up = true,
                    Button::DPadDown => mi.down = true,
                    Button::DPadLeft => mi.left = true,
                    Button::DPadRight => mi.right = true,
                    Button::Start | Button::South => mi.confirm = true,
                    Button::East => mi.back = true,
                    _ => {}
                }
            }
        }
    }

    mi
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

        match emulator_state {
            EmulatorState::Menu(ref mut menu) => {
                // Update cursor blink (~500ms at 60fps)
                menu.cursor_timer += 1;
                if menu.cursor_timer >= 30 {
                    menu.cursor_timer = 0;
                    menu.cursor_visible = !menu.cursor_visible;
                }

                let input = poll_menu_input(&window, &mut gilrs);

                let mut action: Option<MenuAction> = None;

                match menu.submenu {
                    None => {
                        if input.up && menu.selected > 0 {
                            menu.selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                        }
                        if input.down && menu.selected < 2 {
                            menu.selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                        }
                        if input.confirm {
                            match menu.selected {
                                0 => {
                                    menu.submenu = Some(SubMenu::FileBrowser(FileBrowser::new()));
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                }
                                1 => {
                                    menu.submenu = Some(SubMenu::RecentGames { selected: 0 });
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                }
                                2 => {
                                    menu.submenu = Some(SubMenu::Settings { selected: 0 });
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                }
                                _ => {}
                            }
                        }
                        if input.back {
                            break;
                        }
                    }
                    Some(SubMenu::RecentGames { ref mut selected }) => {
                        let count = config.recent_games.len();
                        if count > 0 {
                            if input.up && *selected > 0 {
                                *selected -= 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                            }
                            if input.down && *selected < count - 1 {
                                *selected += 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                            }
                            if input.confirm {
                                let path = config.recent_games[*selected].clone();
                                if Path::new(&path).exists() {
                                    action = Some(MenuAction::LoadRom(path));
                                }
                            }
                        }
                        if input.back {
                            menu.submenu = None;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                        }
                    }
                    Some(SubMenu::Settings { ref mut selected }) => {
                        if input.up && *selected > 0 {
                            *selected -= 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
                        }
                        if input.down && *selected < 2 {
                            *selected += 1;
                            menu.cursor_timer = 0;
                            menu.cursor_visible = true;
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
                        }
                        if input.back {
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
                                if browser.selected < browser.scroll_offset {
                                    browser.scroll_offset = browser.selected;
                                }
                            }
                            if input.down && browser.selected < count - 1 {
                                browser.selected += 1;
                                menu.cursor_timer = 0;
                                menu.cursor_visible = true;
                                if browser.selected >= browser.scroll_offset + 20 {
                                    browser.scroll_offset = browser.selected - 19;
                                }
                            }
                            if input.confirm {
                                let entry_is_dir = browser.entries[browser.selected].is_dir;
                                let entry_path = browser.entries[browser.selected].full_path.clone();
                                if entry_is_dir {
                                    browser.navigate_to(&entry_path);
                                    menu.cursor_timer = 0;
                                    menu.cursor_visible = true;
                                } else {
                                    action = Some(MenuAction::LoadRom(
                                        entry_path.to_string_lossy().to_string(),
                                    ));
                                }
                            }
                        }
                        if input.back || input.backspace {
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
                                        let msg = format!("{}", e);
                                        if let Some(SubMenu::FileBrowser(ref mut browser)) = menu.submenu {
                                            browser.error_message = Some(msg.clone());
                                            browser.error_timer = 120;
                                        }
                                        eprintln!("ROM Error: {}", msg);
                                    }
                                }
                            }
                            Err(e) => {
                                let msg = format!("READ ERROR: {}", e);
                                if let Some(SubMenu::FileBrowser(ref mut browser)) = menu.submenu {
                                    browser.error_message = Some(msg.clone());
                                    browser.error_timer = 120;
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
                    None => render_main_menu(&mut menu_framebuffer, menu),
                    Some(SubMenu::RecentGames { selected }) => {
                        render_recent_games(&mut menu_framebuffer, &config, selected, menu.cursor_visible);
                    }
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
                    // Run one frame of emulation
                    loop {
                        cpu.clock(bus);
                        bus.tick(1);
                        bus.tick_apu();

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

                    window
                        .update_with_buffer(&composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
                        .expect("Failed to update window");

                    // Mouse click handling for console interactions
                    if let Some((mx, my)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
                        let mx = mx as usize;
                        let my = my as usize;

                        let mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
                        let mouse_clicked = mouse_down && !mouse_was_down;
                        mouse_was_down = mouse_down;

                        if mouse_clicked && mx < WINDOW_WIDTH && my < WINDOW_HEIGHT {
                            let console_w = 1000;
                            let console_x = (WINDOW_WIDTH - console_w) / 2;
                            let body_y = TV_HEIGHT + 20;

                            // Cartridge slot
                            let slot_lx = console_w / 2 - 160;
                            let slot_x = console_x + slot_lx;
                            let slot_y = body_y + 8;
                            let slot_w = 320;
                            let slot_h = 34;
                            if mx >= slot_x && mx < slot_x + slot_w && my >= slot_y && my < slot_y + slot_h {
                                let mut ms = MenuState::new();
                                ms.submenu = Some(SubMenu::FileBrowser(FileBrowser::new()));
                                next_state = Some(EmulatorState::Menu(ms));
                            }

                            // Reset button hit test
                            let rst_lx = console_w - 170 + 30;
                            let rst_x = console_x + rst_lx;
                            let rst_y = body_y + 68;
                            let rst_w = 80;
                            let rst_h = 22;
                            if mx >= rst_x && mx < rst_x + rst_w && my >= rst_y && my < rst_y + rst_h {
                                cpu.reset(bus);
                                println!("CPU Reset");
                            }
                        }
                    }

                    frame_counter = frame_counter.wrapping_add(1);
                    let (start_held, select_held) = handle_input(&window, bus, &mut gilrs, frame_counter);

                    // Gamepad quit combo: hold Start+Select for ~1 second (60 frames)
                    if start_held && select_held {
                        quit_hold_frames += 1;
                        if quit_hold_frames >= 60 {
                            game_bus = None;
                            game_cpu = None;
                            quit_hold_frames = 0;
                            emulator_state = EmulatorState::Menu(MenuState::new());
                            continue;
                        }
                    } else {
                        quit_hold_frames = 0;
                    }

                    if window.is_key_pressed(Key::F1, KeyRepeat::No) {
                        crt_enabled = !crt_enabled;
                        config.crt_enabled = crt_enabled;
                        save_config(&config);
                    }

                    // Escape returns to menu immediately (destroys game state)
                    if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                        game_bus = None;
                        game_cpu = None;
                        quit_hold_frames = 0;
                        emulator_state = EmulatorState::Menu(MenuState::new());
                        continue;
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

    let tv_x1: usize = 20;
    let tv_y1: usize = 10;
    let tv_x2 = TV_WIDTH - 20;
    let tv_y2 = TV_HEIGHT - 10;
    let tv_w = tv_x2 - tv_x1;
    let tv_h = tv_y2 - tv_y1;
    let corner_r: usize = 30;

    // Speaker dimensions
    let spk_w: usize = 70;
    let spk_h: usize = 500;
    let spk_y1 = tv_y1 + (tv_h - spk_h) / 2;
    let spk_y2 = spk_y1 + spk_h;
    let left_spk_x1: usize = tv_x1 + 30;
    let left_spk_x2 = left_spk_x1 + spk_w;
    let right_spk_x2: usize = tv_x2 - 30;
    let right_spk_x1 = right_spk_x2 - spk_w;

    // Bottom control panel
    let ctrl_y = SCREEN_Y + SCREEN_H + 6;
    let ctrl_h: usize = tv_y2 - ctrl_y - 4;

    // Button positions in control panel
    let btn_r: usize = 8;
    let btn_y_center = ctrl_y + ctrl_h / 2;
    let btn_spacing: usize = 50;
    let btn_start_x = SCREEN_X + 30;

    // LED position
    let led_cx = btn_start_x - 15;
    let led_cy = btn_y_center;

    // IR receiver
    let ir_x = SCREEN_X + SCREEN_W / 2 + 200;
    let ir_y = btn_y_center - 2;

    // ===== WALL BACKGROUND — warm dark wall =====
    for y in 0..TV_HEIGHT {
        for x in 0..WINDOW_WIDTH {
            let idx = y * WINDOW_WIDTH + x;
            let noise = ((x.wrapping_mul(7) ^ y.wrapping_mul(13)) % 5) as u32;
            let base_r = 0x1Eu32 + noise;
            let base_g = 0x1Cu32 + noise;
            let base_b = 0x1Au32 + noise;
            frame[idx] = (base_r << 16) | (base_g << 8) | base_b;
        }
    }

    // ===== TV BODY (silver-grey rounded rectangle) =====
    for y in tv_y1..tv_y2 {
        for x in tv_x1..tv_x2 {
            let idx = y * WINDOW_WIDTH + x;
            let lx = x - tv_x1;
            let ly = y - tv_y1;

            // Rounded corners with 30px radius
            if (lx < corner_r && ly < corner_r && sq_dist(lx, ly, corner_r, corner_r) > corner_r * corner_r)
                || (lx >= tv_w - corner_r && ly < corner_r && sq_dist(lx, ly, tv_w - corner_r, corner_r) > corner_r * corner_r)
                || (lx < corner_r && ly >= tv_h - corner_r && sq_dist(lx, ly, corner_r, tv_h - corner_r) > corner_r * corner_r)
                || (lx >= tv_w - corner_r && ly >= tv_h - corner_r && sq_dist(lx, ly, tv_w - corner_r, tv_h - corner_r) > corner_r * corner_r)
            {
                continue;
            }

            // Silver-grey gradient: #B0B0B8 top → #858590 bottom
            let gy = ly as f32 / tv_h as f32;
            let r_base = (0xB0u32 as f32 * (1.0 - gy) + 0x85u32 as f32 * gy) as u32;
            let g_base = (0xB0u32 as f32 * (1.0 - gy) + 0x85u32 as f32 * gy) as u32;
            let b_base = (0xB8u32 as f32 * (1.0 - gy) + 0x90u32 as f32 * gy) as u32;

            // Plastic texture noise (1-2 value variance)
            let grain = ((x ^ y ^ (x >> 1) ^ (y >> 2)) % 3) as i32 - 1;

            let mut r = (r_base as i32 + grain).max(0) as u32;
            let mut g = (g_base as i32 + grain).max(0) as u32;
            let mut b = (b_base as i32 + grain).max(0) as u32;

            // 3D bevel: top/left brighter, bottom/right darker (4px)
            if ly < 4 {
                let boost = (4 - ly) as u32 * 5;
                r += boost; g += boost; b += boost;
            }
            if ly >= tv_h - 4 {
                let dim = (ly - (tv_h - 4)) as u32 * 6;
                r = r.saturating_sub(dim); g = g.saturating_sub(dim); b = b.saturating_sub(dim);
            }
            if lx < 4 {
                let boost = (4 - lx) as u32 * 4;
                r += boost; g += boost; b += boost;
            }
            if lx >= tv_w - 4 {
                let dim = (lx - (tv_w - 4)) as u32 * 5;
                r = r.saturating_sub(dim); g = g.saturating_sub(dim); b = b.saturating_sub(dim);
            }

            // Convex curvature shading — brighter at center, darker at edges
            let cx = (x as f32 - (tv_x1 as f32 + tv_w as f32 / 2.0)) / (tv_w as f32 / 2.0);
            let cy_curve = (y as f32 - (tv_y1 as f32 + tv_h as f32 / 2.0)) / (tv_h as f32 / 2.0);
            let curvature = (cx * cx + cy_curve * cy_curve).sqrt().min(1.0);
            let curve_brightness = 1.08 - curvature * 0.16;
            r = (r as f32 * curve_brightness).min(255.0) as u32;
            g = (g as f32 * curve_brightness).min(255.0) as u32;
            b = (b as f32 * curve_brightness).min(255.0) as u32;

            // Upper-left highlight band (simulates overhead light on convex surface)
            let top_dist = ly as f32 / tv_h as f32; // 0 at top, 1 at bottom
            let left_dist = lx as f32 / tv_w as f32; // 0 at left, 1 at right
            if top_dist < 0.08 {
                let band_strength = (1.0 - top_dist / 0.08) * 0.15;
                let center_focus = 1.0 - (cx.abs() * 0.7); // stronger at horizontal center
                let highlight = band_strength * center_focus.max(0.0);
                r = ((r as f32 * (1.0 + highlight)).min(255.0)) as u32;
                g = ((g as f32 * (1.0 + highlight)).min(255.0)) as u32;
                b = ((b as f32 * (1.0 + highlight)).min(255.0)) as u32;
            }
            if left_dist < 0.06 {
                let band_strength = (1.0 - left_dist / 0.06) * 0.10;
                let center_focus = 1.0 - (cy_curve.abs() * 0.7);
                let highlight = band_strength * center_focus.max(0.0);
                r = ((r as f32 * (1.0 + highlight)).min(255.0)) as u32;
                g = ((g as f32 * (1.0 + highlight)).min(255.0)) as u32;
                b = ((b as f32 * (1.0 + highlight)).min(255.0)) as u32;
            }
            // Bottom shadow band
            if top_dist > 0.92 {
                let band_strength = ((top_dist - 0.92) / 0.08) * 0.12;
                r = ((r as f32 * (1.0 - band_strength)).max(0.0)) as u32;
                g = ((g as f32 * (1.0 - band_strength)).max(0.0)) as u32;
                b = ((b as f32 * (1.0 - band_strength)).max(0.0)) as u32;
            }
            // Right edge shadow
            if left_dist > 0.94 {
                let band_strength = ((left_dist - 0.94) / 0.06) * 0.08;
                r = ((r as f32 * (1.0 - band_strength)).max(0.0)) as u32;
                g = ((g as f32 * (1.0 - band_strength)).max(0.0)) as u32;
                b = ((b as f32 * (1.0 - band_strength)).max(0.0)) as u32;
            }

            // ===== SCREEN AREA (rectangular with rounded corners) =====
            let scr_corner_r = 12usize;
            let inset_width = 8usize;
            // Check if pixel is in the screen+inset region
            let sx = x as i32 - SCREEN_X as i32;
            let sy = y as i32 - SCREEN_Y as i32;
            let sw = SCREEN_W as i32;
            let sh = SCREEN_H as i32;
            // Expanded region including inset border
            let ex = sx + inset_width as i32;
            let ey = sy + inset_width as i32;
            let ew = sw + inset_width as i32 * 2;
            let eh = sh + inset_width as i32 * 2;
            if ex >= 0 && ex < ew && ey >= 0 && ey < eh {
                let ex = ex as usize;
                let ey = ey as usize;
                let ew = ew as usize;
                let eh = eh as usize;
                let outer_r = scr_corner_r + inset_width;
                // Rounded corner check for outer inset boundary
                let in_outer_corner =
                    (ex < outer_r && ey < outer_r && sq_dist(ex, ey, outer_r, outer_r) > outer_r * outer_r)
                    || (ex >= ew - outer_r && ey < outer_r && sq_dist(ex, ey, ew - 1 - outer_r, outer_r) > outer_r * outer_r)
                    || (ex < outer_r && ey >= eh - outer_r && sq_dist(ex, ey, outer_r, eh - 1 - outer_r) > outer_r * outer_r)
                    || (ex >= ew - outer_r && ey >= eh - outer_r && sq_dist(ex, ey, ew - 1 - outer_r, eh - 1 - outer_r) > outer_r * outer_r);
                if !in_outer_corner {
                    // Check if inside actual screen opening
                    if sx >= 0 && sx < sw && sy >= 0 && sy < sh {
                        let sxu = sx as usize;
                        let syu = sy as usize;
                        let in_screen_corner =
                            (sxu < scr_corner_r && syu < scr_corner_r && sq_dist(sxu, syu, scr_corner_r, scr_corner_r) > scr_corner_r * scr_corner_r)
                            || (sxu >= SCREEN_W - scr_corner_r && syu < scr_corner_r && sq_dist(sxu, syu, SCREEN_W - 1 - scr_corner_r, scr_corner_r) > scr_corner_r * scr_corner_r)
                            || (sxu < scr_corner_r && syu >= SCREEN_H - scr_corner_r && sq_dist(sxu, syu, scr_corner_r, SCREEN_H - 1 - scr_corner_r) > scr_corner_r * scr_corner_r)
                            || (sxu >= SCREEN_W - scr_corner_r && syu >= SCREEN_H - scr_corner_r && sq_dist(sxu, syu, SCREEN_W - 1 - scr_corner_r, SCREEN_H - 1 - scr_corner_r) > scr_corner_r * scr_corner_r);
                        if !in_screen_corner {
                            // Inside screen opening — black (game pixels composited later)
                            frame[idx] = 0x000000;
                            continue;
                        }
                    }
                    // In the inset border (or screen rounded corner)
                    // Distance from inner screen edge (approximate)
                    let dx_inner = if sx < 0 { (-sx) as usize } else if sx >= sw { (sx - sw + 1) as usize } else { 0 };
                    let dy_inner = if sy < 0 { (-sy) as usize } else if sy >= sh { (sy - sh + 1) as usize } else { 0 };
                    let dist_from_screen = if dx_inner > 0 && dy_inner > 0 {
                        ((dx_inner * dx_inner + dy_inner * dy_inner) as f32).sqrt() as usize
                    } else {
                        dx_inner.max(dy_inner)
                    };
                    // Catch-light at outer edge of inset (1px bright)
                    if dist_from_screen >= inset_width - 1 {
                        frame[idx] = 0x444444;
                        continue;
                    }
                    // Dark inset gradient: near-black innermost 2px, then #101010 fading out
                    let v = if dist_from_screen < 2 {
                        0x05u32
                    } else {
                        let t = (dist_from_screen - 2) as f32 / (inset_width - 3) as f32;
                        (0x05 as f32 + t * (0x10 - 0x05) as f32) as u32
                    };
                    frame[idx] = (v << 16) | (v << 8) | v;
                    continue;
                }
            }

            // ===== LEFT SPEAKER =====
            if x >= left_spk_x1 && x < left_spk_x2 && y >= spk_y1 && y < spk_y2 {
                let bx = x - left_spk_x1;
                let by = y - spk_y1;
                let bw = spk_w;
                let bh = spk_h;

                // Inset border (3px) — darker top/left, lighter bottom/right
                if bx < 3 || by < 3 {
                    frame[idx] = 0x606068;
                    continue;
                }
                if bx >= bw - 3 || by >= bh - 3 {
                    frame[idx] = 0x8A8A90;
                    continue;
                }

                // Horizontal line pattern (speaker mesh) with curvature shading
                let spk_cx = (bx as f32 / bw as f32 - 0.5) * 2.0;
                let spk_cy = (by as f32 / bh as f32 - 0.5) * 2.0;
                let spk_curve = (spk_cx * spk_cx + spk_cy * spk_cy).sqrt().min(1.0);
                let spk_bright = 1.10 - spk_curve * 0.20;
                if by % 4 < 2 {
                    let base = 0x70u32;
                    let v = (base as f32 * spk_bright).min(255.0) as u32;
                    let vb = (0x78u32 as f32 * spk_bright).min(255.0) as u32;
                    frame[idx] = (v << 16) | (v << 8) | vb;
                } else {
                    let base = 0x60u32;
                    let v = (base as f32 * spk_bright).min(255.0) as u32;
                    let vb = (0x68u32 as f32 * spk_bright).min(255.0) as u32;
                    frame[idx] = (v << 16) | (v << 8) | vb;
                }
                continue;
            }

            // ===== RIGHT SPEAKER =====
            if x >= right_spk_x1 && x < right_spk_x2 && y >= spk_y1 && y < spk_y2 {
                let bx = x - right_spk_x1;
                let by = y - spk_y1;
                let bw = spk_w;
                let bh = spk_h;

                // Inset border
                if bx < 3 || by < 3 {
                    frame[idx] = 0x606068;
                    continue;
                }
                if bx >= bw - 3 || by >= bh - 3 {
                    frame[idx] = 0x8A8A90;
                    continue;
                }

                // Horizontal line pattern with curvature shading
                let spk_cx = (bx as f32 / bw as f32 - 0.5) * 2.0;
                let spk_cy = (by as f32 / bh as f32 - 0.5) * 2.0;
                let spk_curve = (spk_cx * spk_cx + spk_cy * spk_cy).sqrt().min(1.0);
                let spk_bright = 1.10 - spk_curve * 0.20;
                if by % 4 < 2 {
                    let base = 0x70u32;
                    let v = (base as f32 * spk_bright).min(255.0) as u32;
                    let vb = (0x78u32 as f32 * spk_bright).min(255.0) as u32;
                    frame[idx] = (v << 16) | (v << 8) | vb;
                } else {
                    let base = 0x60u32;
                    let v = (base as f32 * spk_bright).min(255.0) as u32;
                    let vb = (0x68u32 as f32 * spk_bright).min(255.0) as u32;
                    frame[idx] = (v << 16) | (v << 8) | vb;
                }
                continue;
            }

            // ===== BOTTOM CONTROL PANEL =====
            if y >= ctrl_y && y < ctrl_y + ctrl_h && x >= SCREEN_X - 20 && x < SCREEN_X + SCREEN_W + 20 {
                // Slightly recessed band
                let cy = y - ctrl_y;
                let recess = if cy < 2 { 20u32 } else if cy >= ctrl_h - 2 { 0 } else { 10 };
                let pr = r.saturating_sub(recess);
                let pg = g.saturating_sub(recess);
                let pb = b.saturating_sub(recess);

                // Check for buttons (6 circular buttons)
                let mut is_button = false;
                for i in 0..6usize {
                    let bcx = btn_start_x + i * btn_spacing;
                    let bcy = btn_y_center;
                    let bdx = x as i32 - bcx as i32;
                    let bdy = y as i32 - bcy as i32;
                    let bdist = bdx * bdx + bdy * bdy;
                    let btn_r_sq = (btn_r * btn_r) as i32;
                    if bdist <= btn_r_sq {
                        // Button face — dark grey with slight 3D
                        let depth = (btn_r_sq - bdist) as f32 / btn_r_sq as f32;
                        let bv = (0x50 as f32 + depth * 20.0) as u32;
                        frame[idx] = (bv << 16) | (bv << 8) | bv;
                        is_button = true;
                        break;
                    }
                    // Button rim (2px outside)
                    if bdist <= ((btn_r + 2) * (btn_r + 2)) as i32 {
                        frame[idx] = 0x404048;
                        is_button = true;
                        break;
                    }
                }
                if is_button { continue; }

                // Power LED (green dot near first button)
                let ldx = x as i32 - led_cx as i32;
                let ldy = y as i32 - led_cy as i32;
                let led_dist = ldx * ldx + ldy * ldy;
                if led_dist <= 6 {
                    frame[idx] = 0x00DD44;
                    continue;
                } else if led_dist <= 36 {
                    let t = (36 - led_dist) as f32 / 36.0;
                    let glow = (t * 20.0) as u32;
                    let gr = pr.saturating_sub(glow / 3);
                    let gg = (pg + glow).min(255);
                    let gb = pb.saturating_sub(glow / 4);
                    frame[idx] = (gr.min(255) << 16) | (gg.min(255) << 8) | gb.min(255);
                    continue;
                }

                // IR receiver window (dark red rectangle)
                if x >= ir_x && x < ir_x + 15 && y >= ir_y && y < ir_y + 5 {
                    frame[idx] = 0x3A0000;
                    continue;
                }

                // Brand badge area (centered, subtle embossed rectangle)
                let badge_w: usize = 120;
                let badge_h: usize = 18;
                let badge_x = SCREEN_X + SCREEN_W / 2 - badge_w / 2;
                let badge_y = btn_y_center - badge_h / 2;
                if x >= badge_x && x < badge_x + badge_w && y >= badge_y && y < badge_y + badge_h {
                    let bx = x - badge_x;
                    let by = y - badge_y;
                    if bx == 0 || by == 0 {
                        frame[idx] = ((pr + 8).min(255) << 16) | ((pg + 8).min(255) << 8) | (pb + 8).min(255);
                    } else if bx == badge_w - 1 || by == badge_h - 1 {
                        frame[idx] = (pr.saturating_sub(8) << 16) | (pg.saturating_sub(8) << 8) | pb.saturating_sub(8);
                    } else {
                        frame[idx] = (pr.saturating_sub(3) << 16) | (pg.saturating_sub(3) << 8) | pb.saturating_sub(3);
                    }
                    continue;
                }

                frame[idx] = (pr.min(255) << 16) | (pg.min(255) << 8) | pb.min(255);
                continue;
            }

            frame[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
        }
    }

    // ===== DROP SHADOW below TV =====
    for y in tv_y2..(tv_y2 + 20).min(TV_HEIGHT) {
        for x in tv_x1 + 20..tv_x2 - 20 {
            let idx = y * WINDOW_WIDTH + x;
            let t = (y - tv_y2) as f32 / 20.0;
            let shadow = ((1.0 - t) * 22.0) as u32;
            let existing = frame[idx];
            let er = ((existing >> 16) & 0xFF).saturating_sub(shadow);
            let eg = ((existing >> 8) & 0xFF).saturating_sub(shadow);
            let eb = (existing & 0xFF).saturating_sub(shadow);
            frame[idx] = (er << 16) | (eg << 8) | eb;
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

fn build_console_overlay(frame: &mut Vec<u32>, tv_height: usize, window_width: usize, window_height: usize) {
    let console_y = tv_height;
    let console_w: usize = 1000;
    let console_h: usize = 140; // body starts 20px below console_y, so 160-20=140
    let console_x = (window_width - console_w) / 2;
    let body_y = console_y + 20;
    let body_b = body_y + console_h;

    // Surface/shelf — dark matte
    for y in console_y..window_height {
        for x in 0..window_width {
            frame[y * window_width + x] = 0x1A1A1A;
        }
    }

    // Console body — sleek two-tone design
    let top_h: usize = 50;
    for y in body_y..body_b {
        for x in console_x..console_x + console_w {
            let idx = y * window_width + x;
            let lx = x - console_x;
            let ly = y - body_y;

            // Rounded corners (8px)
            let cr: usize = 8;
            if (lx < cr && ly < cr && sq_dist(lx, ly, cr, cr) > cr * cr)
                || (lx >= console_w - cr && ly < cr && sq_dist(lx, ly, console_w - cr, cr) > cr * cr)
                || (lx < cr && ly >= console_h - cr && sq_dist(lx, ly, cr, console_h - cr) > cr * cr)
                || (lx >= console_w - cr && ly >= console_h - cr && sq_dist(lx, ly, console_w - cr, console_h - cr) > cr * cr)
            {
                continue;
            }

            if ly < top_h {
                // === DARK TOP STRIPE (cartridge area) ===
                let grad = (ly as f32 / top_h as f32 * 6.0) as u32;
                let mut c: u32 = 0x2C + grad;
                if ly == 0 { c = 0x3C; }
                if ly == top_h - 1 { c = 0x20; }

                // Cartridge slot — centered recessed rectangle
                let slot_w: usize = 320;
                let slot_x = console_w / 2 - slot_w / 2;
                if lx >= slot_x && lx < slot_x + slot_w && ly >= 8 && ly < 42 {
                    c = 0x0A;
                    // Slot border
                    if ly == 8 || ly == 41 || lx == slot_x || lx == slot_x + slot_w - 1 { c = 0x04; }
                    // Cartridge body visible inside
                    if lx >= slot_x + 25 && lx < slot_x + slot_w - 25 && ly >= 10 && ly < 40 {
                        let cart_grad = ((ly - 10) as f32 / 30.0 * 12.0) as u32;
                        c = 0x60u32.saturating_sub(cart_grad);
                        // Label stripe on cartridge
                        if lx >= slot_x + 65 && lx < slot_x + slot_w - 65 && ly >= 15 && ly < 35 {
                            let lg = ((ly - 15) as u32 * 2).min(20);
                            let cr = 0xC8u32.saturating_sub(lg);
                            let cg = 0x96u32.saturating_sub(lg);
                            let cb = 0x22u32;
                            if ly == 15 || ly == 34 || lx == slot_x + 65 || lx == slot_x + slot_w - 66 {
                                frame[idx] = 0x886611;
                            } else {
                                frame[idx] = (cr << 16) | (cg << 8) | cb;
                            }
                            continue;
                        }
                    }
                }

                frame[idx] = (c << 16) | (c << 8) | c;
            } else {
                // === LIGHT BOTTOM BODY ===
                let grad = ((ly - top_h) as f32 / (console_h - top_h) as f32 * 10.0) as u32;
                let base = 0xB8u32.saturating_sub(grad);
                let mut r = base;
                let mut g = base;
                let mut b = (base as f32 * 0.97) as u32;

                if ly == top_h { r = 0xCE; g = 0xCE; b = 0xCC; }
                if ly >= console_h - 2 { r = 0x85; g = 0x85; b = 0x83; }
                if lx < 3 || lx >= console_w - 3 {
                    r = r.saturating_sub(12);
                    g = g.saturating_sub(12);
                    b = b.saturating_sub(12);
                }

                // === POWER LED (left side) ===
                let led_cx: usize = 48;
                let led_cy: usize = 75;
                if lx >= led_cx.saturating_sub(5) && lx <= led_cx + 5 && ly >= led_cy.saturating_sub(5) && ly <= led_cy + 5 {
                    let dx = lx as i32 - led_cx as i32;
                    let dy = ly as i32 - led_cy as i32;
                    let d = dx * dx + dy * dy;
                    if d <= 12 { frame[idx] = 0x00DD55; continue; }
                    if d <= 32 { frame[idx] = 0x003D15; continue; }
                }

                // === POWER BUTTON (pill, left) ===
                let btn_x: usize = 35;
                let btn_y: usize = 68;
                let btn_w: usize = 65;
                let btn_h: usize = 20;
                if lx >= btn_x + 20 && lx < btn_x + 20 + btn_w && ly >= btn_y && ly < btn_y + btn_h {
                    let bx = lx - (btn_x + 20);
                    let by = ly - btn_y;
                    let pill_r = btn_h / 2;
                    let in_pill = if bx < pill_r { sq_dist(bx, by, pill_r, pill_r) <= pill_r * pill_r }
                        else if bx >= btn_w - pill_r { sq_dist(bx, by, btn_w - pill_r, pill_r) <= pill_r * pill_r }
                        else { true };
                    if in_pill {
                        let mut bc = 0x58u32;
                        if by < 2 { bc = 0x6C; }
                        if by >= btn_h - 2 { bc = 0x40; }
                        frame[idx] = (bc << 16) | (bc << 8) | bc;
                        continue;
                    }
                }

                // Divider after power section
                let pwr_div_x: usize = 150;
                if lx == pwr_div_x && ly >= 55 && ly < console_h - 8 {
                    r = 0x96; g = 0x96; b = 0x94;
                }

                // === RESET BUTTON (pill, right side) ===
                let rst_section_x = console_w - 170;
                let rst_x = rst_section_x + 30;
                let rst_y: usize = 68;
                let rst_w: usize = 80;
                let rst_h: usize = 22;
                if lx >= rst_x && lx < rst_x + rst_w && ly >= rst_y && ly < rst_y + rst_h {
                    let bx = lx - rst_x;
                    let by = ly - rst_y;
                    let pill_r = rst_h / 2;
                    let in_pill = if bx < pill_r { sq_dist(bx, by, pill_r, pill_r) <= pill_r * pill_r }
                        else if bx >= rst_w - pill_r { sq_dist(bx, by, rst_w - pill_r, pill_r) <= pill_r * pill_r }
                        else { true };
                    if in_pill {
                        let mut bc = 0x66u32;
                        if by < 2 { bc = 0x7C; }
                        if by >= rst_h - 2 { bc = 0x4E; }
                        frame[idx] = (bc << 16) | (bc << 8) | bc;
                        continue;
                    }
                }

                // Divider before reset
                if lx == rst_section_x && ly >= 55 && ly < console_h - 8 {
                    r = 0x96; g = 0x96; b = 0x94;
                }

                frame[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
            }
        }
    }

    // Drop shadow under console
    for y in body_b..body_b + 8 {
        if y >= window_height { break; }
        for x in console_x + 5..console_x + console_w - 5 {
            let idx = y * window_width + x;
            let shadow = ((body_b + 8 - y) as u32 * 3).min(20);
            let existing = frame[idx];
            let er = ((existing >> 16) & 0xFF).saturating_sub(shadow);
            let eg = ((existing >> 8) & 0xFF).saturating_sub(shadow);
            let eb = (existing & 0xFF).saturating_sub(shadow);
            frame[idx] = (er << 16) | (eg << 8) | eb;
        }
    }

    // Text labels
    let body_y_offset = body_y;
    draw_text(frame, "POWER", console_x + 45, body_y_offset + 93, 0x686868, window_width);
    draw_text(frame, "RESET", console_x + console_w - 130, body_y_offset + 93, 0x686868, window_width);
    draw_text(frame, "CLICK TO INSERT CARTRIDGE", console_x + console_w / 2 - 50, body_y_offset + 45, 0x3E3E3E, window_width);

    // Controller port labels
    let ports_total_w = 80 * 2 + 40;
    let port1_lx = (console_w - ports_total_w) / 2;
    let port2_lx = port1_lx + 80 + 40;
    draw_text(frame, "1P", console_x + port1_lx + 35, body_y_offset + 137, 0x686868, window_width);
    draw_text(frame, "2P", console_x + port2_lx + 35, body_y_offset + 137, 0x686868, window_width);
}
