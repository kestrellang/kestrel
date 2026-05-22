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

    // Compile C sources to object files
    let tmp_dir = std::env::temp_dir();
    let mut c_objects: Vec<std::path::PathBuf> = Vec::new();
    for (i, c_src) in options.c_sources.iter().enumerate() {
        let c_obj = tmp_dir.join(format!("kestrel_c{}_{}.o", std::process::id(), i));
        let c_output = Command::new(&cc)
            .arg("-c")
            .arg(c_src)
            .arg("-o")
            .arg(&c_obj)
            .output()
            .map_err(|e| CodegenError::LinkerError(format!("failed to compile {}: {e}", c_src.display())))?;
        if !c_output.status.success() {
            let stderr = String::from_utf8_lossy(&c_output.stderr);
            return Err(CodegenError::LinkerError(format!("failed to compile {}: {stderr}", c_src.display())));
        }
        c_objects.push(c_obj);
    }

    let mut cmd = Command::new(&cc);
    cmd.arg(object_path);
    for c_obj in &c_objects {
        cmd.arg(c_obj);
    }
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

    // Clean up C object files
    for c_obj in &c_objects {
        let _ = std::fs::remove_file(c_obj);
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodegenError::LinkerError(format!(
            "linker failed with exit code {:?}:\n{stderr}",
            output.status.code()
        )));
    }

    Ok(())
}
