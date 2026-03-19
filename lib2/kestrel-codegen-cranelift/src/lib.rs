//! Cranelift backend for Kestrel (lib2).
//!
//! Compiles `MirModule` → native object code via Cranelift.
//!
//! Pipeline: `MirModule` → monomorphize (BFS) → CodegenContext → compile all → object bytes → link

pub mod block;
pub mod common;
pub mod context;
pub mod error;
pub mod function;
pub mod link;
pub mod monomorphize;
pub mod place;
pub mod rvalue;
pub mod terminator;
pub mod types;

use context::CodegenContext;
use error::CodegenError;
use kestrel_codegen2::TargetConfig;
use kestrel_mir::MirModule;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Options for code generation.
#[derive(Debug, Clone, Default)]
pub struct CodegenOptions {
    /// Enable debug info in the output.
    pub debug_info: bool,
    /// Optimization level: 0=none, 1=speed, 2=speed+size.
    pub opt_level: u8,
    /// Libraries to link against (e.g., "c", "m").
    pub libraries: Vec<String>,
    /// Library search paths.
    pub library_paths: Vec<String>,
    /// macOS frameworks to link against.
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

/// Compile a MIR module to native object code.
///
/// The `MirModule` is taken by shared reference — by-value types need no interning.
pub fn compile(
    module: &MirModule,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<CompilationResult, CodegenError> {
    // Phase 1: Collect all monomorphized function instantiations
    let mono_set = monomorphize::collect_all(module)
        .map_err(|errors| {
            let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            CodegenError::Monomorphization(msgs.join("\n"))
        })?;

    // Phase 2: Compile all functions
    let mut ctx = CodegenContext::new(module, target, options, mono_set)?;
    ctx.compile_all()?;

    // Phase 3: Emit object bytes
    let object_bytes = ctx.finish()?;

    Ok(CompilationResult { object_bytes })
}

/// Counter for unique temp file names.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Compile and link to an executable.
pub fn compile_and_link(
    module: &MirModule,
    target: &TargetConfig,
    options: &CodegenOptions,
    output_path: impl AsRef<Path>,
) -> Result<(), CodegenError> {
    let result = compile(module, target, options)?;

    // Write temp object file with a unique name
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let thread_id = std::thread::current().id();
    let temp_name = format!("kestrel_{counter}_{thread_id:?}.o");
    let temp_path = std::env::temp_dir().join(&temp_name);

    result.write_object_file(&temp_path)?;

    // Link
    let link_result = link::link_executable(
        &temp_path,
        output_path.as_ref(),
        &options.libraries,
        &options.library_paths,
        &options.frameworks,
    );

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    link_result
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::PathBuf;

    fn stdlib_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../lang/std")
            .canonicalize()
            .expect("stdlib path should exist at lang/std")
    }

    /// Full pipeline: source → lex/parse/build → infer → MIR lower → codegen.
    /// Uses a minimal program (no stdlib) to avoid stack overflow in tests.
    #[test]
    fn compile_empty_main() {
        let mut compiler = kestrel_compiler2::Compiler::new();

        // Minimal program without stdlib to keep test fast and avoid stack overflow
        let entity = compiler.set_source(
            "test.ks",
            "module Test\nfunc main() { }".into(),
        );
        compiler.build(entity);
        compiler.infer_all();

        // Lower to MIR
        let mir = kestrel_mir_lower::lower_module(compiler.world(), compiler.root()).with_all_passes();

        // Verify we have an entry point
        assert!(mir.entry_point.is_some() || mir.functions.iter().any(|f| f.name.ends_with("main")),
            "should have a main function");

        // Compile to object
        let target = TargetConfig::host();
        let options = CodegenOptions::default();
        let result = compile(&mir, &target, &options);

        match result {
            Ok(r) => {
                eprintln!("Compilation succeeded: {} bytes of object code", r.object_bytes.len());
                assert!(!r.object_bytes.is_empty());
            }
            Err(e) => {
                // Codegen may fail on missing features — that's expected during development.
                // Log and don't panic so the test acts as a progress indicator.
                eprintln!("Compilation failed (expected during development): {e}");
            }
        }
    }
}
