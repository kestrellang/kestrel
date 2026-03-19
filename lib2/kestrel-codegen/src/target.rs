//! Target configuration and host detection.
//!
//! Wraps `target-lexicon` to provide the pointer width needed by layout
//! computation and code generation backends.

use std::str::FromStr;
use target_lexicon::{PointerWidth, Triple};

/// Target configuration for code generation.
///
/// Backends use `pointer_size()` and `triple` to emit correct machine code.
/// Layout computation uses `pointer_size()` for pointer/reference types.
#[derive(Debug, Clone)]
pub struct TargetConfig {
    /// The full target triple (e.g., `aarch64-apple-darwin`).
    pub triple: Triple,
    /// Pointer width in bytes (4 or 8).
    pointer_width: u8,
}

impl TargetConfig {
    /// Create a target config for the host system.
    pub fn host() -> Self {
        let triple = Triple::host();
        let pointer_width = match triple.pointer_width() {
            Ok(PointerWidth::U16) => 2,
            Ok(PointerWidth::U32) => 4,
            Ok(PointerWidth::U64) => 8,
            Err(_) => 8, // default to 64-bit
        };
        Self {
            triple,
            pointer_width,
        }
    }

    /// Create a target config from a target triple string.
    pub fn from_triple(triple: &str) -> Result<Self, String> {
        let triple =
            Triple::from_str(triple).map_err(|e| format!("invalid target triple: {e}"))?;
        let pointer_width = match triple.pointer_width() {
            Ok(PointerWidth::U16) => 2,
            Ok(PointerWidth::U32) => 4,
            Ok(PointerWidth::U64) => 8,
            Err(_) => return Err(format!("unknown pointer width for triple: {triple}")),
        };
        Ok(Self {
            triple,
            pointer_width,
        })
    }

    /// Returns true if this is a 64-bit target.
    pub fn is_64bit(&self) -> bool {
        self.pointer_width == 8
    }

    /// Pointer size in bytes as `u64`.
    ///
    /// Uses `u64` (not `usize`) so layout computation is correct when
    /// cross-compiling for a 64-bit target on a 32-bit host.
    pub fn pointer_size(&self) -> u64 {
        self.pointer_width as u64
    }
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self::host()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_target() {
        let target = TargetConfig::host();
        // Host should be either 32-bit or 64-bit
        assert!(target.pointer_size() == 4 || target.pointer_size() == 8);
    }

    #[test]
    fn from_triple_x86_64() {
        let target = TargetConfig::from_triple("x86_64-unknown-linux-gnu").unwrap();
        assert!(target.is_64bit());
        assert_eq!(target.pointer_size(), 8);
    }

    #[test]
    fn from_triple_aarch64() {
        let target = TargetConfig::from_triple("aarch64-apple-darwin").unwrap();
        assert!(target.is_64bit());
        assert_eq!(target.pointer_size(), 8);
    }

    #[test]
    fn from_triple_32bit() {
        let target = TargetConfig::from_triple("i686-unknown-linux-gnu").unwrap();
        assert!(!target.is_64bit());
        assert_eq!(target.pointer_size(), 4);
    }

    #[test]
    fn invalid_triple() {
        let result = TargetConfig::from_triple("not-a-real-triple");
        assert!(result.is_err());
    }
}
