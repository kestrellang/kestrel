//! Linker integration.

use crate::error::CodegenError;
use crate::CodegenOptions;

use std::path::Path;
use std::process::Command;

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

    let mut cmd = Command::new(&cc);
    cmd.arg(object_path).arg("-o").arg(output_path);

    // Add library search paths (-L)
    for path in &options.library_paths {
        cmd.arg(format!("-L{}", path));
    }

    // Add libraries (-l)
    // Supports both library names (e.g., "ssl") and literal filenames (e.g., ":libfoo.a")
    for lib in &options.libraries {
        cmd.arg(format!("-l{}", lib));
    }

    // Add frameworks (macOS -framework)
    for framework in &options.frameworks {
        cmd.arg("-framework").arg(framework);
    }

    let status = cmd
        .status()
        .map_err(|e| CodegenError::LinkerError(format!("failed to run {}: {}", cc, e)))?;

    if !status.success() {
        return Err(CodegenError::LinkerError(format!(
            "{} exited with status {}",
            cc,
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}
