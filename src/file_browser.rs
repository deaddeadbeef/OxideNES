use crate::rom_library::default_rom_library_dir;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileBrowserEntry {
    pub name: String,
    pub is_dir: bool,
    pub full_path: PathBuf,
    pub size_kb: u32,
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<FileBrowserEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub error_message: Option<String>,
    pub error_timer: u32,
}

impl FileBrowser {
    pub fn new(start_dir: Option<&str>) -> Self {
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

    pub fn default_dir() -> PathBuf {
        let roms_dir = default_rom_library_dir();
        let home = env::var("USERPROFILE")
            .or_else(|_| env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let downloads = PathBuf::from(home).join("Downloads");
        if roms_dir.is_dir() {
            roms_dir
        } else if downloads.is_dir() {
            downloads
        } else {
            PathBuf::from(".")
        }
    }

    pub fn navigate_to(&mut self, dir: &Path) {
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

pub fn scan_directory(dir: &Path) -> Vec<FileBrowserEntry> {
    scan_directory_result(dir).unwrap_or_default()
}

pub fn scan_directory_result(dir: &Path) -> Result<Vec<FileBrowserEntry>, std::io::Error> {
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
            } else {
                0
            };
            files.push(FileBrowserEntry {
                name,
                is_dir: false,
                full_path: path,
                size_kb,
            });
        }
    }

    files.sort_by_key(|entry| entry.name.to_lowercase());
    dirs.sort_by_key(|entry| entry.name.to_lowercase());

    files.extend(dirs);
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = env::temp_dir().join(format!("oxidenes_browser_{name}_{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scan_directory_lists_nes_files_then_dirs_case_insensitively() {
        let dir = temp_dir("scan");
        fs::write(dir.join("zeta.NES"), vec![0u8; 2048]).unwrap();
        fs::write(dir.join("alpha.nes"), vec![0u8; 1024]).unwrap();
        fs::write(dir.join("notes.txt"), b"not a rom").unwrap();
        fs::create_dir_all(dir.join("beta_dir")).unwrap();
        fs::create_dir_all(dir.join("AlphaDir")).unwrap();

        let entries = scan_directory_result(&dir).unwrap();
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();

        assert_eq!(names, vec!["alpha.nes", "zeta.NES", "AlphaDir", "beta_dir"]);
        assert!(!entries[0].is_dir);
        assert_eq!(entries[0].size_kb, 1);
        assert!(entries[2].is_dir);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn navigate_to_resets_selection_and_clears_prior_error() {
        let dir = temp_dir("navigate");
        fs::write(dir.join("game.nes"), [0u8; 16]).unwrap();
        let mut browser = FileBrowser {
            current_dir: PathBuf::from("."),
            entries: Vec::new(),
            selected: 9,
            scroll_offset: 4,
            error_message: Some("ACCESS DENIED".to_string()),
            error_timer: 30,
        };

        browser.navigate_to(&dir);

        assert_eq!(browser.current_dir, dir);
        assert_eq!(browser.entries.len(), 1);
        assert_eq!(browser.selected, 0);
        assert_eq!(browser.scroll_offset, 0);
        assert!(browser.error_message.is_none());
        assert_eq!(browser.error_timer, 0);
        let _ = fs::remove_dir_all(&browser.current_dir);
    }
}
