//! Linker integration.

use crate::error::CodegenError;
use kestrel_codegen::TargetConfig;

use std::path::Path;
use std::process::Command;

/// Link an object file into an executable.
pub fn link_executable(
    object_path: &Path,
    output_path: &Path,
    target: &TargetConfig,
) -> Result<(), CodegenError> {
    // Use the system C compiler as linker
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    let status = Command::new(&cc)
        .arg(object_path)
        .arg("-o")
        .arg(output_path)
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
