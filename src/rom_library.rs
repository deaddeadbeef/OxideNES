use crate::config::{config_dir, EmulatorConfig};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomImportMode {
    Copy,
    Symlink,
}

impl RomImportMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "copy" => Some(Self::Copy),
            "symlink" | "link" => Some(Self::Symlink),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Symlink => "symlink",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomImportSummary {
    pub source_dir: PathBuf,
    pub target_dir: PathBuf,
    pub mode: RomImportMode,
    pub imported: usize,
    pub skipped_existing: usize,
    pub skipped_entries: usize,
}

#[derive(Debug)]
pub enum RomImportError {
    SourceNotDirectory(PathBuf),
    SourceIsTarget(PathBuf),
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for RomImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceNotDirectory(path) => {
                write!(f, "source is not a directory: {}", path.display())
            }
            Self::SourceIsTarget(path) => {
                write!(f, "source is already the import target: {}", path.display())
            }
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
        }
    }
}

impl Error for RomImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn default_rom_library_dir() -> PathBuf {
    config_dir().join("roms")
}

pub fn point_config_at_default_library(config: &mut EmulatorConfig) -> PathBuf {
    let dir = default_rom_library_dir();
    config.rom_directory = Some(dir.to_string_lossy().to_string());
    dir
}

pub fn import_rom_folder(
    source_dir: impl AsRef<Path>,
    mode: RomImportMode,
) -> Result<RomImportSummary, RomImportError> {
    import_rom_folder_to(source_dir, default_rom_library_dir(), mode)
}

pub fn import_rom_folder_and_configure_library(
    config: &mut EmulatorConfig,
    source_dir: impl AsRef<Path>,
    mode: RomImportMode,
) -> Result<RomImportSummary, RomImportError> {
    let summary = import_rom_folder(source_dir, mode)?;
    point_config_at_default_library(config);
    Ok(summary)
}

pub fn import_rom_folder_to(
    source_dir: impl AsRef<Path>,
    target_dir: impl AsRef<Path>,
    mode: RomImportMode,
) -> Result<RomImportSummary, RomImportError> {
    let source_dir = source_dir.as_ref();
    let target_dir = target_dir.as_ref();

    if !source_dir.is_dir() {
        return Err(RomImportError::SourceNotDirectory(source_dir.to_path_buf()));
    }

    let source_canonical = canonicalize_path(source_dir)?;
    if target_dir.exists() {
        let target_canonical = canonicalize_path(target_dir)?;
        if source_canonical == target_canonical {
            return Err(RomImportError::SourceIsTarget(source_dir.to_path_buf()));
        }
    }

    fs::create_dir_all(target_dir).map_err(|source| RomImportError::Io {
        path: target_dir.to_path_buf(),
        source,
    })?;

    let mut roms = Vec::new();
    let mut skipped_entries = 0;
    for entry in fs::read_dir(&source_canonical).map_err(|source| RomImportError::Io {
        path: source_dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| RomImportError::Io {
            path: source_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if is_nes_rom(&path) {
            roms.push(path);
        } else {
            skipped_entries += 1;
        }
    }

    roms.sort_by_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    });

    let mut imported = 0;
    let mut skipped_existing = 0;
    for source_path in roms {
        let Some(file_name) = source_path.file_name() else {
            skipped_entries += 1;
            continue;
        };
        let target_path = target_dir.join(file_name);
        if fs::symlink_metadata(&target_path).is_ok() {
            skipped_existing += 1;
            continue;
        }

        match mode {
            RomImportMode::Copy => {
                fs::copy(&source_path, &target_path).map_err(|source| RomImportError::Io {
                    path: target_path.clone(),
                    source,
                })?;
            }
            RomImportMode::Symlink => {
                create_file_symlink(&source_path, &target_path).map_err(|source| {
                    RomImportError::Io {
                        path: target_path.clone(),
                        source,
                    }
                })?;
            }
        }
        imported += 1;
    }

    Ok(RomImportSummary {
        source_dir: source_canonical,
        target_dir: target_dir.to_path_buf(),
        mode,
        imported,
        skipped_existing,
        skipped_entries,
    })
}

fn canonicalize_path(path: &Path) -> Result<PathBuf, RomImportError> {
    path.canonicalize().map_err(|source| RomImportError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn is_nes_rom(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("nes"))
}

#[cfg(windows)]
fn create_file_symlink(source: &Path, target: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_file(source, target)
}

#[cfg(unix)]
fn create_file_symlink(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(not(any(unix, windows)))]
fn create_file_symlink(_source: &Path, _target: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "file symlinks are not supported on this platform",
    ))
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
        let dir = std::env::temp_dir().join(format!("oxidenes_rom_import_{name}_{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn mode_parser_accepts_copy_and_symlink_aliases() {
        assert_eq!(RomImportMode::parse("copy"), Some(RomImportMode::Copy));
        assert_eq!(
            RomImportMode::parse("Symlink"),
            Some(RomImportMode::Symlink)
        );
        assert_eq!(RomImportMode::parse("link"), Some(RomImportMode::Symlink));
        assert_eq!(RomImportMode::parse("move"), None);
    }

    #[test]
    fn default_library_uses_config_roms_directory() {
        assert_eq!(default_rom_library_dir(), config_dir().join("roms"));
    }

    #[test]
    fn import_copy_copies_only_nes_files_without_overwriting_existing_targets() {
        let root = temp_dir("copy");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("zeta.NES"), b"zeta").unwrap();
        fs::write(source.join("alpha.nes"), b"alpha").unwrap();
        fs::write(source.join("notes.txt"), b"not a rom").unwrap();
        fs::create_dir_all(source.join("subdir")).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("alpha.nes"), b"existing").unwrap();

        let summary = import_rom_folder_to(&source, &target, RomImportMode::Copy).unwrap();

        assert_eq!(summary.mode, RomImportMode::Copy);
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.skipped_existing, 1);
        assert_eq!(summary.skipped_entries, 2);
        assert_eq!(fs::read(target.join("alpha.nes")).unwrap(), b"existing");
        assert_eq!(fs::read(target.join("zeta.NES")).unwrap(), b"zeta");
        assert!(!target.join("notes.txt").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn import_symlink_links_nes_files_when_platform_allows_it() {
        let root = temp_dir("symlink");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("game.nes"), b"linked").unwrap();

        match import_rom_folder_to(&source, &target, RomImportMode::Symlink) {
            Ok(summary) => {
                assert_eq!(summary.imported, 1);
                assert_eq!(fs::read(target.join("game.nes")).unwrap(), b"linked");
            }
            Err(RomImportError::Io { source, .. })
                if matches!(
                    source.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::Unsupported
                ) =>
            {
                let _ = fs::remove_dir_all(root);
                return;
            }
            Err(error) => panic!("unexpected symlink import error: {error}"),
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn import_rejects_source_equal_to_target() {
        let source = temp_dir("same");
        fs::write(source.join("game.nes"), b"rom").unwrap();

        let error = import_rom_folder_to(&source, &source, RomImportMode::Copy).unwrap_err();

        assert!(matches!(error, RomImportError::SourceIsTarget(_)));
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn point_config_at_default_library_sets_rom_directory() {
        let mut config = EmulatorConfig::default();

        let dir = point_config_at_default_library(&mut config);

        assert_eq!(dir, default_rom_library_dir());
        assert_eq!(
            config.rom_directory.as_deref(),
            Some(default_rom_library_dir().to_string_lossy().as_ref())
        );
    }

    #[test]
    fn import_and_configure_preserves_existing_root_on_failure() {
        let missing_source = temp_dir("missing_source").join("does-not-exist");
        let mut config = EmulatorConfig {
            rom_directory: Some("existing-library".to_string()),
            ..EmulatorConfig::default()
        };

        let result = import_rom_folder_and_configure_library(
            &mut config,
            &missing_source,
            RomImportMode::Copy,
        );

        assert!(matches!(result, Err(RomImportError::SourceNotDirectory(_))));
        assert_eq!(config.rom_directory.as_deref(), Some("existing-library"));
        if let Some(root) = missing_source.parent() {
            let _ = fs::remove_dir_all(root);
        }
    }
}
