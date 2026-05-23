use crate::rendering::CrtConfig;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn default_region() -> String {
    "ntsc".to_string()
}

fn default_glass_intensity() -> u8 {
    60
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyBindings {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub a: String,
    pub b: String,
    pub start: String,
    pub select: String,
    pub turbo_a: String,
    pub turbo_b: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyboardBindings {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub a: String,
    pub b: String,
    pub start: String,
    pub select: String,
    pub turbo_a: String,
    pub turbo_b: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ControllerBindings {
    pub a: String,
    pub b: String,
    pub turbo_a: String,
    pub turbo_b: String,
    pub start: String,
    pub select: String,
    pub deadzone: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InputBindings {
    pub keyboard_p1: KeyboardBindings,
    pub keyboard_p2: KeyboardBindings,
    pub controller_p1: ControllerBindings,
    pub controller_p2: ControllerBindings,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EmulatorConfig {
    pub recent_games: Vec<String>,
    pub crt_enabled: bool,
    pub barrel_distortion: bool,
    pub audio_volume: u32,
    #[serde(default)]
    pub key_bindings: Option<KeyBindings>,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default)]
    pub input_bindings: InputBindings,
    #[serde(default = "default_glass_intensity")]
    pub glass_intensity: u8,
    #[serde(default)]
    pub config_version: u32,
    #[serde(default)]
    pub crt_config: CrtConfig,
    #[serde(default = "default_true")]
    pub check_for_updates: bool,
    #[serde(default)]
    pub favorite_games: Vec<String>,
    #[serde(default)]
    pub rom_directory: Option<String>,
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

pub fn config_dir() -> PathBuf {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".nes-emulator")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_config() -> EmulatorConfig {
    load_config_from_path(&config_path())
}

pub fn load_config_from_path(path: &Path) -> EmulatorConfig {
    if path.exists() {
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(cfg) = serde_json::from_str::<EmulatorConfig>(&data) {
                let (cfg, migrated) = migrate_config(cfg);
                if migrated {
                    save_config_to_path(&cfg, path);
                }
                return cfg;
            }
        }
    }

    let cfg = EmulatorConfig::default();
    save_config_to_path(&cfg, path);
    cfg
}

fn migrate_config(mut cfg: EmulatorConfig) -> (EmulatorConfig, bool) {
    let mut migrated = false;

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
        cfg.config_version = 3;
        migrated = true;
    }

    (cfg, migrated)
}

pub fn save_config(cfg: &EmulatorConfig) {
    save_config_to_path(cfg, &config_path());
}

pub fn save_config_to_path(cfg: &EmulatorConfig, path: &Path) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(data) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(path, data);
    }
}

pub fn add_recent_game(cfg: &mut EmulatorConfig, path: &str) {
    cfg.recent_games.retain(|p| p != path);
    cfg.recent_games.insert(0, path.to_string());
    cfg.recent_games.truncate(10);
}

pub fn toggle_favorite(config: &mut EmulatorConfig, path: &str) -> bool {
    if let Some(pos) = config.favorite_games.iter().position(|g| g == path) {
        config.favorite_games.remove(pos);
        false
    } else {
        config.favorite_games.push(path.to_string());
        true
    }
}

pub fn is_favorite(config: &EmulatorConfig, path: &str) -> bool {
    config.favorite_games.iter().any(|g| g == path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        env::temp_dir()
            .join(format!("oxidenes_config_{name}_{nonce}"))
            .join("config.json")
    }

    #[test]
    fn add_recent_game_deduplicates_and_caps_history() {
        let mut cfg = EmulatorConfig::default();
        for i in 0..12 {
            add_recent_game(&mut cfg, &format!("rom_{i}.nes"));
        }
        add_recent_game(&mut cfg, "rom_5.nes");

        assert_eq!(
            cfg.recent_games.first().map(String::as_str),
            Some("rom_5.nes")
        );
        assert_eq!(cfg.recent_games.len(), 10);
        assert_eq!(
            cfg.recent_games
                .iter()
                .filter(|p| p.as_str() == "rom_5.nes")
                .count(),
            1
        );
    }

    #[test]
    fn toggle_favorite_adds_and_removes_path() {
        let mut cfg = EmulatorConfig::default();

        assert!(toggle_favorite(&mut cfg, "test.nes"));
        assert!(is_favorite(&cfg, "test.nes"));
        assert!(!toggle_favorite(&mut cfg, "test.nes"));
        assert!(!is_favorite(&cfg, "test.nes"));
    }

    #[test]
    fn load_config_migrates_v1_key_bindings_and_persists_v3() {
        let path = temp_config_path("migration");
        let dir = path.parent().expect("config dir").to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        let old = r#"{
            "recent_games": [],
            "crt_enabled": true,
            "barrel_distortion": false,
            "audio_volume": 100,
            "key_bindings": {
                "up": "I", "down": "K", "left": "J", "right": "L",
                "a": "A", "b": "B", "start": "Enter", "select": "Space",
                "turbo_a": "T", "turbo_b": "Y"
            },
            "config_version": 1
        }"#;
        fs::write(&path, old).unwrap();

        let cfg = load_config_from_path(&path);

        assert_eq!(cfg.config_version, 3);
        assert_eq!(cfg.input_bindings.keyboard_p1.up, "I");
        assert_eq!(cfg.input_bindings.keyboard_p1.select, "Space");
        assert!(cfg.key_bindings.is_none());
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("\"config_version\": 3"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_config_writes_default_when_missing() {
        let path = temp_config_path("missing");
        let dir = path.parent().expect("config dir").to_path_buf();

        let cfg = load_config_from_path(&path);

        assert_eq!(cfg.config_version, 3);
        assert!(path.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
