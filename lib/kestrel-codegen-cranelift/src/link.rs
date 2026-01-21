//! Linker integration.

use crate::CodegenOptions;
use crate::error::CodegenError;

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn compile_malloc_debug(cc: &str) -> Result<Option<std::path::PathBuf>, CodegenError> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let source_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("runtime")
        .join("malloc_debug.c");
    if !source_path.exists() {
        return Ok(None);
    }

    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let object_path = std::env::temp_dir().join(format!(
        "kestrel_malloc_debug_{}_{}.o",
        std::process::id(),
        unique_id
    ));

    let status = Command::new(cc)
        .arg("-c")
        .arg(&source_path)
        .arg("-o")
        .arg(&object_path)
        .status()
        .map_err(|e| {
            CodegenError::LinkerError(format!(
                "failed to compile {}: {}",
                source_path.display(),
                e
            ))
        })?;

    if !status.success() {
        return Err(CodegenError::LinkerError(format!(
            "{} exited with status {} while compiling {}",
            cc,
            status.code().unwrap_or(-1),
            source_path.display()
        )));
    }

    Ok(Some(object_path))
}

/// Link an object file into an executable.
///
/// Uses the system C compiler (or `CC` environment variable) as the linker.
/// Supports library linking via `-l`, library search paths via `-L`,
/// and macOS frameworks via `-framework`.
pub fn link_executable(
    object_path: &Path,
    output_path: &Path,
    options: &CodegenOptions,
) -> Result<(), CodegenError> {
    // Use the system C compiler as linker
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    let debug_object = compile_malloc_debug(&cc)?;

    let mut cmd = Command::new(&cc);
    cmd.arg(object_path).arg("-o").arg(output_path);

    if let Some(path) = &debug_object {
        cmd.arg(path);
    }

    // Add library search paths (-L)
    for path in &options.library_paths {
        cmd.arg(format!("-L{}", path));
    }

    // Add libraries (-l) and object files
    // Supports library names (e.g., "ssl") and literal filenames starting with ":"
    // If it starts with ":" and ends with ".o" or ".a", pass it directly as an object/archive file
    for lib in &options.libraries {
        if let Some(path) = lib.strip_prefix(':') {
            // Strip the leading ":" and pass the file directly
            cmd.arg(path);
        } else {
            cmd.arg(format!("-l{}", lib));
        }
    }

    // Add frameworks (macOS -framework)
    for framework in &options.frameworks {
        cmd.arg("-framework").arg(framework);
    }

    let status = cmd.status().map_err(|e| {
        if let Some(path) = &debug_object {
            let _ = std::fs::remove_file(path);
        }
        CodegenError::LinkerError(format!("failed to run {}: {}", cc, e))
    })?;

    if let Some(path) = &debug_object {
        let _ = std::fs::remove_file(path);
    }

    if !status.success() {
        return Err(CodegenError::LinkerError(format!(
            "{} exited with status {}",
            cc,
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}
