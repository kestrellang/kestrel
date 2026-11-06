//! The compilation target, shared by every stage that depends on it.
//!
//! One `TargetConfig` is built from the target triple (or the host) and
//! threaded through the pipeline, so `@platform` filtering in the AST
//! builder, MIR layout, and both codegen backends all agree on the OS and
//! pointer width. Keep target facts here rather than re-deriving them in a
//! stage: a second copy is how layout and codegen drift apart.

use std::str::FromStr;
use target_lexicon::{Architecture, OperatingSystem, PointerWidth, Triple, Vendor};

/// The target being compiled for.
#[derive(Debug, Clone)]
pub struct TargetConfig {
    /// The full target triple (e.g., `aarch64-apple-darwin`).
    pub triple: Triple,
    /// Pointer width in bytes (2, 4 or 8).
    pointer_width: u8,
    /// The OS `@platform(...)` filters against. `None` for an OS Kestrel has
    /// no `@platform` case for, which disables `@platform` filtering.
    os: Option<Os>,
}

impl TargetConfig {
    /// The host system.
    pub fn host() -> Self {
        let triple = Triple::host();
        // The host triple always has a known width; default to 64-bit if not.
        let pointer_width = pointer_width_of(&triple).unwrap_or(8);
        Self::new(triple, pointer_width)
    }

    /// Parse a target triple string such as `x86_64-unknown-linux-gnu`.
    pub fn from_triple(triple: &str) -> Result<Self, String> {
        let triple = Triple::from_str(triple).map_err(|e| format!("invalid target triple: {e}"))?;
        let pointer_width = pointer_width_of(&triple)
            .ok_or_else(|| format!("unknown pointer width for triple: {triple}"))?;
        Ok(Self::new(triple, pointer_width))
    }

    fn new(triple: Triple, pointer_width: u8) -> Self {
        let os = Os::of(&triple);
        Self {
            triple,
            pointer_width,
            os,
        }
    }

    /// Returns true if this is a 64-bit target.
    pub fn is_64bit(&self) -> bool {
        self.pointer_width == 8
    }

    /// Whether this targets WebAssembly, which needs its own LLVM target,
    /// static relocation, and a generic CPU.
    pub fn is_wasm(&self) -> bool {
        matches!(
            self.triple.architecture,
            Architecture::Wasm32 | Architecture::Wasm64
        )
    }

    /// Whether this is the machine the compiler runs on, so code can be
    /// tuned for the host CPU.
    pub fn is_host(&self) -> bool {
        self.triple == Triple::host()
    }

    /// Pointer size in bytes as `u64`.
    ///
    /// Uses `u64` (not `usize`) so layout computation is correct when
    /// cross-compiling for a 64-bit target on a 32-bit host.
    pub fn pointer_size(&self) -> u64 {
        self.pointer_width as u64
    }

    /// The OS for `@platform` filtering, if Kestrel knows it.
    pub fn os(&self) -> Option<Os> {
        self.os
    }

    /// The C calling convention's rules for passing structs by value.
    pub fn c_abi(&self) -> CAbi {
        match self.triple.architecture {
            Architecture::X86_64 if self.triple.operating_system != OperatingSystem::Windows => {
                CAbi::SysV64
            },
            Architecture::Aarch64(_) => CAbi::Aapcs64,
            Architecture::Wasm32 | Architecture::Wasm64 => CAbi::Wasm,
            _ => CAbi::Other,
        }
    }
}

/// Which C ABI rules decide how a struct crosses an `@extern`/`@export` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CAbi {
    /// x86-64 System V (Linux, macOS on Intel).
    SysV64,
    /// AArch64 AAPCS64 (Linux, Apple silicon).
    Aapcs64,
    /// WebAssembly (clang's basic C ABI): a struct holding a single scalar
    /// passes and returns as that scalar; any other struct passes by pointer
    /// to a copy and returns through a hidden pointer.
    Wasm,
    /// Anything else: structs pass by pointer and return through a hidden
    /// pointer, which is right for no real C ABI but keeps the old behaviour.
    Other,
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self::host()
    }
}

fn pointer_width_of(triple: &Triple) -> Option<u8> {
    match triple.pointer_width().ok()? {
        PointerWidth::U16 => Some(2),
        PointerWidth::U32 => Some(4),
        PointerWidth::U64 => Some(8),
    }
}

/// Target operating system for `@platform(.Darwin)` / `@platform(.Linux)` /
/// `@platform(.Emscripten)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Os {
    Darwin,
    Linux,
    /// WebAssembly in a browser through Emscripten (`wasm64-unknown-emscripten`).
    /// Its libc is musl, so most constants match Linux; struct layouts don't.
    Emscripten,
}

impl Os {
    /// Parse the case name written in `@platform(...)`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            // Lowercase is the pre-0.18 spelling (warned by `AttrsOf`).
            "Darwin" | "darwin" => Some(Os::Darwin),
            "Linux" | "linux" => Some(Os::Linux),
            "Emscripten" => Some(Os::Emscripten),
            _ => None,
        }
    }

    /// The host OS. Panics on a platform Kestrel does not support.
    pub fn host() -> Self {
        match std::env::consts::OS {
            "macos" => Os::Darwin,
            "linux" => Os::Linux,
            other => panic!("unsupported platform: {}", other),
        }
    }

    /// The `@platform` OS a triple names: any Apple triple is Darwin.
    fn of(triple: &Triple) -> Option<Self> {
        if triple.vendor == Vendor::Apple {
            return Some(Os::Darwin);
        }
        match triple.operating_system {
            OperatingSystem::Darwin | OperatingSystem::MacOSX { .. } => Some(Os::Darwin),
            OperatingSystem::Linux => Some(Os::Linux),
            OperatingSystem::Emscripten => Some(Os::Emscripten),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_target() {
        let target = TargetConfig::host();
        assert!(target.pointer_size() == 4 || target.pointer_size() == 8);
        assert_eq!(target.os(), Some(Os::host()));
    }

    #[test]
    fn from_triple_x86_64_linux() {
        let target = TargetConfig::from_triple("x86_64-unknown-linux-gnu").unwrap();
        assert!(target.is_64bit());
        assert_eq!(target.pointer_size(), 8);
        assert_eq!(target.os(), Some(Os::Linux));
    }

    #[test]
    fn from_triple_aarch64_darwin() {
        let target = TargetConfig::from_triple("aarch64-apple-darwin").unwrap();
        assert!(target.is_64bit());
        assert_eq!(target.pointer_size(), 8);
        assert_eq!(target.os(), Some(Os::Darwin));
    }

    #[test]
    fn from_triple_32bit() {
        let target = TargetConfig::from_triple("i686-unknown-linux-gnu").unwrap();
        assert!(!target.is_64bit());
        assert_eq!(target.pointer_size(), 4);
    }

    #[test]
    fn from_triple_wasm64_emscripten() {
        let target = TargetConfig::from_triple("wasm64-unknown-emscripten").unwrap();
        assert_eq!(target.pointer_size(), 8);
        assert_eq!(target.os(), Some(Os::Emscripten));
        assert!(target.is_wasm());
    }

    #[test]
    fn unknown_os_disables_platform_filtering() {
        let target = TargetConfig::from_triple("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(target.os(), None);
    }

    #[test]
    fn invalid_triple() {
        assert!(TargetConfig::from_triple("not-a-real-triple").is_err());
    }
}
