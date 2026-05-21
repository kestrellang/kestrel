use std::path::Path;
use std::process::Command;

use kestrel_codegen::TargetConfig;
use target_lexicon::OperatingSystem;

use crate::error::CodegenError;
use crate::CodegenOptions;

/// Link an object file into an executable.
pub fn link_executable(
    object_path: &Path,
    output_path: &Path,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<(), CodegenError> {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    let mut cmd = Command::new(&cc);
    cmd.arg(object_path);
    cmd.arg("-o");
    cmd.arg(output_path);

    for path in &options.library_paths {
        cmd.arg(format!("-L{path}"));
    }

    for lib in &options.libraries {
        if let Some(path) = lib.strip_prefix(':') {
            cmd.arg(path);
        } else {
            cmd.arg(format!("-l{lib}"));
        }
    }

    for framework in &options.frameworks {
        cmd.arg("-framework");
        cmd.arg(framework);
    }

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
