//! System linker invocation.
//!
//! Uses the system C compiler (cc) as the linker, with support for
//! library paths, libraries, and macOS frameworks.

use crate::error::CodegenError;
use kestrel_codegen::target::TargetConfig;
use std::path::Path;
use std::process::Command;
use target_lexicon::OperatingSystem;

/// Link an object file into an executable.
pub fn link_executable(
    object_path: &Path,
    output_path: &Path,
    target: &TargetConfig,
    libraries: &[String],
    library_paths: &[String],
    frameworks: &[String],
) -> Result<(), CodegenError> {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    let mut cmd = Command::new(&cc);
    cmd.arg(object_path);
    cmd.arg("-o");
    cmd.arg(output_path);

    // Library search paths
    for path in library_paths {
        cmd.arg(format!("-L{path}"));
    }

    // Libraries
    for lib in libraries {
        if let Some(path) = lib.strip_prefix(':') {
            // Literal path (strip colon prefix)
            cmd.arg(path);
        } else {
            cmd.arg(format!("-l{lib}"));
        }
    }

    // macOS frameworks
    for framework in frameworks {
        cmd.arg("-framework");
        cmd.arg(framework);
    }

    // Linux requires explicit -lm for math functions (on macOS they're in libSystem)
    if !matches!(target.triple.operating_system, OperatingSystem::Darwin) {
        cmd.arg("-lm");
    }

    let output = cmd
        .output()
        .map_err(|e| CodegenError::LinkerError(format!("failed to run linker '{cc}': {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodegenError::LinkerError(format!(
            "linker failed with exit code {:?}:\n{stderr}",
            output.status.code()
        )));
    }

    Ok(())
}
