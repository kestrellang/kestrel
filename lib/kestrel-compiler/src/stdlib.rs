//! Standard Library Configuration
//!
//! Handles locating and loading the Kestrel standard library.

use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for standard library loading
#[derive(Debug, Clone)]
pub struct StdLibConfig {
    /// Path to stdlib root directory (None = use default)
    pub path: Option<PathBuf>,
    /// Whether stdlib is enabled
    pub enabled: bool,
}

impl Default for StdLibConfig {
    fn default() -> Self {
        Self {
            path: None,
            enabled: true,
        }
    }
}

impl StdLibConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn disabled() -> Self {
        Self {
            path: None,
            enabled: false,
        }
    }
}

/// Resolved standard library with loaded source files
pub struct StdLib {
    /// (name, content, full_path) triples for all stdlib files
    pub sources: Vec<(String, String, PathBuf)>,
}

impl StdLib {
    /// Load the standard library from configuration
    pub fn load(config: &StdLibConfig) -> Result<Option<Self>, StdLibError> {
        if !config.enabled {
            return Ok(None);
        }

        let path = Self::resolve_path(config)?;
        let sources = Self::load_from_path(&path)?;

        Ok(Some(Self { sources }))
    }

    /// Resolve the stdlib path using search order
    fn resolve_path(config: &StdLibConfig) -> Result<PathBuf, StdLibError> {
        // 1. Explicit path from config
        if let Some(ref path) = config.path {
            if path.exists() {
                return Ok(path.clone());
            }
            return Err(StdLibError::NotFound(path.display().to_string()));
        }

        // 2. Environment variable
        if let Ok(env_path) = std::env::var("KESTREL_STD_PATH") {
            let path = PathBuf::from(env_path);
            if path.exists() {
                return Ok(path);
            }
        }

        // 3. Relative to current directory (development)
        let dev_path = PathBuf::from("lang/std");
        if dev_path.exists() {
            return Ok(dev_path);
        }

        // 4. Relative to executable
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            let installed_path = exe_dir.join("lib/std");
            if installed_path.exists() {
                return Ok(installed_path);
            }
        }

        Err(StdLibError::NotFound("standard library".to_string()))
    }

    /// Recursively load all .ks files from a directory
    fn load_from_path(path: &Path) -> Result<Vec<(String, String, PathBuf)>, StdLibError> {
        let mut sources = Vec::new();
        Self::collect_sources(path, path, &mut sources)?;
        Ok(sources)
    }

    fn collect_sources(
        root: &Path,
        current: &Path,
        sources: &mut Vec<(String, String, PathBuf)>,
    ) -> Result<(), StdLibError> {
        let platform = host_platform();
        let entries = fs::read_dir(current)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::collect_sources(root, &path, sources)?;
            } else if path.extension().is_some_and(|e| e == "ks") {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                if !should_include_file(&filename, platform) {
                    continue;
                }
                let content = fs::read_to_string(&path)?;
                // Use relative path from stdlib root with std/ prefix
                let rel_path = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                // Get the absolute path for file constant resolution
                let full_path = path.canonicalize().unwrap_or_else(|_| path.clone());
                sources.push((format!("std/{}", rel_path), content, full_path));
            }
        }
        Ok(())
    }
}

/// Returns the platform suffix for the current host OS.
fn host_platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

/// Checks whether a `.ks` file should be included for the given platform.
///
/// Files with a platform suffix (e.g., `foo.darwin.ks`, `foo.linux.ks`) are only
/// included when the suffix matches the current platform. Plain `.ks` files are
/// always included.
fn should_include_file(filename: &str, platform: &str) -> bool {
    let stem = match filename.strip_suffix(".ks") {
        Some(s) => s,
        None => return true,
    };

    for known in &["darwin", "linux", "windows"] {
        let suffix = format!(".{}", known);
        if stem.ends_with(&suffix) {
            return *known == platform;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_files_always_included() {
        assert!(should_include_file("libc.ks", "darwin"));
        assert!(should_include_file("libc.ks", "linux"));
    }

    #[test]
    fn test_darwin_files_only_on_darwin() {
        assert!(should_include_file("libc.darwin.ks", "darwin"));
        assert!(!should_include_file("libc.darwin.ks", "linux"));
    }

    #[test]
    fn test_linux_files_only_on_linux() {
        assert!(should_include_file("libc.linux.ks", "linux"));
        assert!(!should_include_file("libc.linux.ks", "darwin"));
    }

    #[test]
    fn test_windows_files_only_on_windows() {
        assert!(should_include_file("os.windows.ks", "windows"));
        assert!(!should_include_file("os.windows.ks", "darwin"));
        assert!(!should_include_file("os.windows.ks", "linux"));
    }

    #[test]
    fn test_non_ks_files_always_included() {
        assert!(should_include_file("readme.md", "darwin"));
        assert!(should_include_file("readme.md", "linux"));
    }
}

#[derive(Debug)]
pub enum StdLibError {
    NotFound(String),
    IoError(std::io::Error),
}

impl From<std::io::Error> for StdLibError {
    fn from(e: std::io::Error) -> Self {
        StdLibError::IoError(e)
    }
}

impl std::fmt::Display for StdLibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StdLibError::NotFound(path) => write!(f, "standard library not found: {}", path),
            StdLibError::IoError(e) => write!(f, "IO error loading stdlib: {}", e),
        }
    }
}

impl std::error::Error for StdLibError {}
