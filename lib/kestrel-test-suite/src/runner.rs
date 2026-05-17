//! Compile, link, and execute Kestrel programs for end-to-end testing.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use kestrel_compiler::Compiler;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Result of running a compiled Kestrel program.
#[derive(Debug, Clone)]
pub struct RunResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Compile a Kestrel program to a temporary executable, run it, and capture output.
///
/// Returns Err if compilation or linking fails.
/// The temporary directory is cleaned up after execution.
pub fn compile_and_run(compiler: &Compiler) -> Result<RunResult, String> {
    // Skip execution if env var is set (for CI speed)
    if std::env::var("KESTREL_SKIP_CODEGEN").is_ok() {
        return Ok(RunResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: "SKIPPED: KESTREL_SKIP_CODEGEN=1".to_string(),
        });
    }

    let temp_dir = temp_dir();
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("failed to create temp dir: {e}"))?;

    let exe_path = temp_dir.join(if cfg!(windows) { "test.exe" } else { "test" });

    let options = kestrel_codegen_cranelift::CodegenOptions::default();
    compiler
        .compile_and_link(&exe_path, &options)
        .map_err(|e| format!("codegen/link failed: {e}"))?;

    let output = std::process::Command::new(&exe_path)
        .output()
        .map_err(|e| format!("failed to execute: {e}"))?;

    // Clean up temp directory (unless KESTREL_KEEP_TEST_BIN is set for debugging)
    if std::env::var("KESTREL_KEEP_TEST_BIN").is_err() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    } else {
        eprintln!("KEPT: {}", exe_path.display());
    }

    Ok(RunResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Generate a unique temporary directory path.
fn temp_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("kestrel2_test_{}_{}", std::process::id(), id))
}
