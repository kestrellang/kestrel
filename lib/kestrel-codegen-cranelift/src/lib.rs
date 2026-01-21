//! Cranelift backend for Kestrel code generation.
//!
//! This crate compiles Kestrel's MIR (from `kestrel-execution-graph`) to native
//! machine code using Cranelift.
//!
//! # Usage
//!
//! ```ignore
//! use kestrel_codegen_cranelift::{compile, CodegenOptions};
//! use kestrel_codegen::TargetConfig;
//!
//! let mir_ctx = /* ... */;
//! let target = TargetConfig::host();
//! let options = CodegenOptions::default();
//!
//! let result = compile(&mir_ctx, &target, &options)?;
//! result.write_object_file("output.o")?;
//! ```

mod block;
mod context;
mod error;
mod function;
mod intrinsics;
mod link;
pub mod monomorphize;
mod place;
mod rvalue;
mod terminator;
mod types;

pub use context::CodegenContext;
pub use error::CodegenError;
pub use link::link_executable;

use kestrel_codegen::TargetConfig;
use kestrel_execution_graph::MirContext;
use std::path::Path;

/// Options for code generation.
#[derive(Debug, Clone, Default)]
pub struct CodegenOptions {
    /// Enable debug info generation.
    pub debug_info: bool,
    /// Optimization level (0 = none, 1 = speed, 2 = speed+size).
    pub opt_level: u8,
    /// Libraries to link with (-l flags).
    /// Supports both library names (e.g., "ssl") and literal filenames (e.g., ":libfoo.a").
    pub libraries: Vec<String>,
    /// Library search paths (-L flags).
    pub library_paths: Vec<String>,
    /// Frameworks to link with (macOS -framework flags).
    pub frameworks: Vec<String>,
}

/// Result of compilation.
pub struct CompilationResult {
    /// The compiled object file bytes.
    pub object_bytes: Vec<u8>,
}

impl CompilationResult {
    /// Write the object file to disk.
    pub fn write_object_file(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        std::fs::write(path, &self.object_bytes)
    }
}

/// Compile MIR to native object code.
///
/// The MIR context is taken as mutable because the collection phase
/// may intern new types during substitution.
pub fn compile(
    mir: &mut MirContext,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<CompilationResult, CodegenError> {
    // Collection phase: discover all instantiations
    let mono_set = monomorphize::collect_all(mir)?;

    // Compilation phase
    let mut ctx = CodegenContext::new(mir, target, options, mono_set)?;
    ctx.compile_all()?;
    let object_bytes = ctx.finish()?;
    Ok(CompilationResult { object_bytes })
}

/// Compile MIR and link to an executable.
pub fn compile_and_link(
    mir: &mut MirContext,
    target: &TargetConfig,
    options: &CodegenOptions,
    output_path: impl AsRef<Path>,
) -> Result<(), CodegenError> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let result = compile(mir, target, options)?;

    // Write object file to temp location with unique name
    // Use atomic counter + thread ID + nanos to ensure uniqueness in parallel tests
    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();
    let object_path = temp_dir.join(format!(
        "kestrel_output_{}_{:?}_{}.o",
        unique_id, thread_id, counter
    ));
    result
        .write_object_file(&object_path)
        .map_err(|e| CodegenError::IoError(e.to_string()))?;

    // Link to executable
    link_executable(&object_path, output_path.as_ref(), options)?;

    // Clean up
    let _ = std::fs::remove_file(&object_path);

    Ok(())
}
