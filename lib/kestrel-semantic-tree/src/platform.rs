//! Target platform for conditional compilation.
//!
//! Used by the `@platform(.darwin)` / `@platform(.linux)` attribute to
//! include or exclude declarations based on the compilation target.

/// Target platform for conditional compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    Darwin,
    Linux,
}

impl TargetPlatform {
    /// Parse a platform name (e.g., "darwin", "linux").
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "darwin" => Some(TargetPlatform::Darwin),
            "linux" => Some(TargetPlatform::Linux),
            _ => None,
        }
    }

    /// Detect the platform of the current host.
    pub fn host() -> Self {
        match std::env::consts::OS {
            "macos" => TargetPlatform::Darwin,
            "linux" => TargetPlatform::Linux,
            other => panic!("unsupported platform: {}", other),
        }
    }

    /// All known platform names, for use in error messages.
    pub fn known_names() -> &'static [&'static str] {
        &["darwin", "linux"]
    }
}
