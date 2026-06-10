//! Compile, link, and execute Kestrel programs for end-to-end testing.
//!
//! Backend selection: a test pins its backends with a `// backends:` header
//! (each listed backend becomes its own trial); unannotated tests follow the
//! `KESTREL_BACKEND` env var (`llvm` selects the LLVM backend; anything else
//! uses the default Cranelift backend). This lets the same suite validate
//! both backends, and lets ABI-sensitive tests demand both unconditionally.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use kestrel_compiler::Compiler;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Which codegen backend a trial compiles with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Cranelift,
    Llvm,
}

impl Backend {
    /// Suite-wide default for tests without a `// backends:` header.
    pub fn default_from_env() -> Self {
        if std::env::var("KESTREL_BACKEND").as_deref() == Ok("llvm") {
            Backend::Llvm
        } else {
            Backend::Cranelift
        }
    }

    /// Header-value spelling (`// backends: cranelift,llvm`).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "cranelift" => Some(Backend::Cranelift),
            "llvm" => Some(Backend::Llvm),
            _ => None,
        }
    }

    /// Trial-name tag for non-primary backends (`foo__llvm.ks`). Must stay
    /// dot-free and the full trial name must keep its `.ks` suffix, or
    /// triage's raw↔identity name round-trip drops the trial.
    pub fn tag(self) -> &'static str {
        match self {
            Backend::Cranelift => "cranelift",
            Backend::Llvm => "llvm",
        }
    }
}

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
pub fn compile_and_run(compiler: &Compiler, backend: Backend) -> Result<RunResult, String> {
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

    let link_result = if backend == Backend::Llvm {
        let options = kestrel_codegen_llvm::CodegenOptions {
            c_sources: stdlib_c_sources(),
            ..Default::default()
        };
        compiler
            .compile_and_link_llvm(&exe_path, &options)
            .map_err(|e| format!("{e}"))
    } else {
        let options = kestrel_codegen_cranelift::CodegenOptions {
            c_sources: stdlib_c_sources(),
            ..Default::default()
        };
        compiler
            .compile_and_link(&exe_path, &options)
            .map_err(|e| format!("{e}"))
    };
    if let Err(e) = link_result {
        let mut msg = format!("codegen/link failed: {e}");
        let diagnostics = compiler.diagnostics();
        if !diagnostics.is_empty() {
            let files = kestrel_compiler::diagnostic::WorldFiles::from_world(
                compiler.world(),
                compiler.files(),
            );
            let config = codespan_reporting::term::Config::default();
            let mut buf = codespan_reporting::term::termcolor::NoColor::new(Vec::new());
            for diag in &diagnostics {
                let _ =
                    codespan_reporting::term::emit_to_write_style(&mut buf, &config, &files, diag);
            }
            if let Ok(rendered) = String::from_utf8(buf.into_inner()) {
                msg.push('\n');
                msg.push_str(&rendered);
            }
        }
        return Err(msg);
    }

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

/// Collect C shim sources from the stdlib directory.
fn stdlib_c_sources() -> Vec<PathBuf> {
    let std_dir = if let Ok(path) = std::env::var("KESTREL_STD") {
        PathBuf::from(path)
    } else {
        let manifest = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("lang/std")
    };
    let shim = std_dir.join("io/libc_shims.c");
    if shim.exists() { vec![shim] } else { vec![] }
}
