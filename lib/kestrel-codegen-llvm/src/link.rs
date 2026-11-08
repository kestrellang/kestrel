//! Object -> executable linking. Backend-agnostic: identical to the Cranelift
//! backend's `link.rs` since both produce a standard `.o` file. Shells out to
//! `$CC` (default `cc`) to compile any C shim sources and link the final image.

use std::path::Path;
use std::process::Command;

use kestrel_codegen::TargetConfig;
use target_lexicon::OperatingSystem;

use std::sync::atomic::Ordering;

use crate::error::CodegenError;
use crate::{CodegenOptions, LINK_COUNTER};

/// Flags the C compiler needs to build and link for `target`, beyond the
/// host defaults. Emscripten picks wasm64 with `-sMEMORY64`; 64-bit is the
/// only width Kestrel's layout supports today.
fn cc_target_flags(target: &TargetConfig) -> Vec<String> {
    match target.triple.operating_system {
        OperatingSystem::Emscripten if target.is_64bit() => vec!["-sMEMORY64=1".to_string()],
        _ => Vec::new(),
    }
}

pub fn link_executable(
    object_path: &Path,
    output_path: &Path,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<(), CodegenError> {
    let emscripten = matches!(target.triple.operating_system, OperatingSystem::Emscripten);
    // Emscripten's `emcc` is the C compiler and linker for the web target.
    let default_cc = if emscripten { "emcc" } else { "cc" };
    let cc = std::env::var("CC").unwrap_or_else(|_| default_cc.to_string());
    let target_flags = cc_target_flags(target);

    let tmp_dir = std::env::temp_dir();
    let mut c_objects: Vec<std::path::PathBuf> = Vec::new();
    for (i, c_src) in options.c_sources.iter().enumerate() {
        let link_id = LINK_COUNTER.fetch_add(1, Ordering::Relaxed);
        let c_obj = tmp_dir.join(format!(
            // `llvm` keeps the name apart from the Cranelift backend's,
            // which counts with its own counter in the same process.
            "kestrel_llvm_c{}_{}_{}.o",
            std::process::id(),
            link_id,
            i
        ));
        let c_output = Command::new(&cc)
            .args(&target_flags)
            .arg("-c")
            .arg(c_src)
            .arg("-o")
            .arg(&c_obj)
            .output()
            .map_err(|e| {
                CodegenError::LinkerError(format!("failed to compile {}: {e}", c_src.display()))
            })?;
        if !c_output.status.success() {
            let stderr = String::from_utf8_lossy(&c_output.stderr);
            return Err(CodegenError::LinkerError(format!(
                "failed to compile {}: {stderr}",
                c_src.display()
            )));
        }
        c_objects.push(c_obj);
    }

    let mut cmd = Command::new(&cc);
    cmd.args(&target_flags);
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

    cmd.args(&options.link_args);

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
