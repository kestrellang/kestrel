//! Target configuration and host detection.

use std::str::FromStr;
use target_lexicon::Triple;

/// Target configuration for code generation.
#[derive(Debug, Clone)]
pub struct TargetConfig {
    /// The target triple (e.g., aarch64-apple-darwin).
    pub triple: Triple,
    /// Pointer width in bytes.
    pub pointer_width: u8,
}

impl TargetConfig {
    /// Create a target config for the host system.
    pub fn host() -> Self {
        let triple = Triple::host();
        let pointer_width = match triple.pointer_width() {
            Ok(pw) => pw.bytes(),
            Err(_) => 8, // Default to 64-bit
        };
        Self {
            triple,
            pointer_width,
        }
    }

    /// Create a target config from a target triple string.
    ///
    /// # Example
    /// ```
    /// use kestrel_codegen::TargetConfig;
    /// let config = TargetConfig::from_triple("x86_64-unknown-linux-gnu").unwrap();
    /// ```
    pub fn from_triple(triple: &str) -> Result<Self, String> {
        let triple = Triple::from_str(triple)
            .map_err(|e| format!("invalid target triple '{}': {}", triple, e))?;
        let pointer_width = triple
            .pointer_width()
            .map_err(|_| format!("unknown pointer width for triple '{}'", triple))?
            .bytes();
        Ok(Self {
            triple,
            pointer_width,
        })
    }

    /// Returns true if this is a 64-bit target.
    pub fn is_64bit(&self) -> bool {
        self.pointer_width == 8
    }

    /// Returns the pointer size in bytes.
    pub fn pointer_size(&self) -> usize {
        self.pointer_width as usize
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
    fn test_host_target() {
        let config = TargetConfig::host();
        // Should be 64-bit on modern systems
        assert!(config.is_64bit());
    }

    #[test]
    fn test_from_triple() {
        let config = TargetConfig::from_triple("x86_64-unknown-linux-gnu").unwrap();
        assert!(config.is_64bit());
        assert_eq!(config.pointer_size(), 8);
    }

    #[test]
    fn test_invalid_triple() {
        let result = TargetConfig::from_triple("invalid-triple");
        assert!(result.is_err());
    }
}
