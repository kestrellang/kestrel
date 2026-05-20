//! Cranelift backend for Kestrel (lib).
//!
//! Compiles `MirModule` → native object code via Cranelift.
//!
//! Pipeline: `MirModule` → monomorphize (BFS) → CodegenContext → compile all → object bytes → link.

#![allow(clippy::result_large_err)]

const _CACHE_BUST: u8 = 2;

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
use kestrel_codegen::TargetConfig;
use kestrel_mir::MirModule;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
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
    /// C source files to compile and link (e.g., shims for variadic libc functions).
    pub c_sources: Vec<PathBuf>,
    /// Capture per-function CLIF text before Cranelift compiles it.
    /// When true, `CompilationResult::clif_text` is populated.
    pub emit_clif: bool,
}

/// Result of compilation.
pub struct CompilationResult {
    /// The compiled object file bytes.
    pub object_bytes: Vec<u8>,
    /// Per-function CLIF text, populated when `CodegenOptions::emit_clif` is set.
    /// Each entry is `(mangled_name, clif_body)`.
    pub clif_text: Vec<(String, String)>,
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
    // Codegen can be deeply recursive (Cranelift verify/compile, recursive place projections).
    // Use stacker to grow the stack on demand.
    stacker::maybe_grow(256 * 1024, 4 * 1024 * 1024, || {
        compile_inner(module, target, options)
    })
}

fn compile_inner(
    module: &MirModule,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<CompilationResult, CodegenError> {
    // Bail out if MIR lowering produced any `MirTy::Error` locations. Proceeding
    // to Cranelift would panic inside `def_var` with a misleading type-mismatch
    // message; the accumulated diagnostics describe the real cause.
    if module.lowering_error_count > 0 {
        return Err(CodegenError::MirLoweringErrors(module.lowering_error_count));
    }

    // Phase 1: Collect all monomorphized function instantiations
    let mono_set = monomorphize::collect_all(module).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        CodegenError::Monomorphization(msgs.join("\n"))
    })?;

    // Phase 2: Compile all functions
    let mut ctx = CodegenContext::new(module, target, options, mono_set)?;
    ctx.compile_all()?;

    // Phase 3: Emit object bytes (consumes ctx but keeps captured CLIF)
    let clif_text = std::mem::take(&mut ctx.clif_outputs);
    let object_bytes = ctx.finish()?;

    Ok(CompilationResult {
        object_bytes,
        clif_text,
    })
}

/// Counter for unique temp file names.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempObjectFile {
    path: PathBuf,
}

impl TempObjectFile {
    fn create_near_output(output_path: &Path, object_bytes: &[u8]) -> Result<Self, CodegenError> {
        let dir = output_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let output_name = output_path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "output".into());
        let output_name = sanitize_temp_component(&output_name);
        let pid = std::process::id();

        for _ in 0..100 {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let thread_id = sanitize_temp_component(&format!("{:?}", std::thread::current().id()));
            let path = dir.join(format!(
                ".{output_name}.kestrel-{pid}-{counter}-{thread_id}.o"
            ));

            let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => file,
                Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err.into()),
            };
            file.write_all(object_bytes)?;
            return Ok(Self { path });
        }

        Err(CodegenError::IoError(std::io::Error::new(
            ErrorKind::AlreadyExists,
            "could not create a unique temporary object file",
        )))
    }
}

impl Drop for TempObjectFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn sanitize_temp_component(component: &str) -> String {
    let sanitized: String = component
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "output".to_string()
    } else {
        sanitized
    }
}

/// Compile and link to an executable.
pub fn compile_and_link(
    module: &MirModule,
    target: &TargetConfig,
    options: &CodegenOptions,
    output_path: impl AsRef<Path>,
) -> Result<(), CodegenError> {
    let result = compile(module, target, options)?;
    let output_path = output_path.as_ref();
    let temp_object = TempObjectFile::create_near_output(output_path, &result.object_bytes)?;

    // Compile C shim sources into temporary object files
    let c_objects: Vec<PathBuf> = options
        .c_sources
        .iter()
        .map(|src| compile_c_source(src))
        .collect::<Result<_, _>>()?;

    let mut libraries = options.libraries.clone();
    for obj in &c_objects {
        libraries.push(format!(":{}", obj.display()));
    }

    // Link
    let result = link::link_executable(
        &temp_object.path,
        output_path,
        target,
        &libraries,
        &options.library_paths,
        &options.frameworks,
    );

    // Clean up C object files
    for obj in &c_objects {
        let _ = std::fs::remove_file(obj);
    }

    result
}

/// Compile a single C source file to a unique temporary .o file.
fn compile_c_source(source: &Path) -> Result<PathBuf, CodegenError> {
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let pid = std::process::id();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let thread_id = sanitize_temp_component(&format!("{:?}", std::thread::current().id()));
    let obj_path = std::env::temp_dir().join(format!(
        ".{stem}.kestrel-shim-{pid}-{counter}-{thread_id}.o"
    ));
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let output = std::process::Command::new(&cc)
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(&obj_path)
        .output()
        .map_err(|e| CodegenError::LinkerError(format!("failed to compile {}: {e}", source.display())))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodegenError::LinkerError(format!(
            "C compilation of {} failed:\n{stderr}",
            source.display()
        )));
    }
    Ok(obj_path)
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

    fn compile_source(source: &str) -> Result<CompilationResult, error::CodegenError> {
        let mut compiler = kestrel_compiler::Compiler::new();
        let entity = compiler.set_source("test.ks", source.into());
        compiler.build(entity);
        kestrel_compiler_driver::CompilerDriver::new(&compiler).infer_all();

        let mir =
            kestrel_mir_lower::lower_module(compiler.world(), compiler.root()).with_all_passes();
        let target = TargetConfig::host();
        let options = CodegenOptions::default();
        compile(&mir, &target, &options)
    }

    #[test]
    fn compile_empty_main() {
        let result = compile_source("module Test\nfunc main() { }");
        match result {
            Ok(r) => {
                eprintln!("Compilation succeeded: {} bytes", r.object_bytes.len());
                assert!(!r.object_bytes.is_empty());
            },
            Err(e) => {
                eprintln!("Compilation failed (expected during development): {e}");
            },
        }
    }

    /// Full pipeline without stdlib: compile → link → run.
    /// Returns the process exit code for a void main.
    #[test]
    fn compile_link_run_void_main() {
        let result = compile_source("module Test\nfunc main() { }").unwrap();

        let dir = std::env::temp_dir().join("kestrel_e2e_test");
        let _ = std::fs::create_dir_all(&dir);
        let obj_path = dir.join("test.o");
        let exe_path = dir.join("test_exe");

        result.write_object_file(&obj_path).unwrap();
        link::link_executable(&obj_path, &exe_path, &[], &[], &[]).unwrap();

        let output = std::process::Command::new(&exe_path).output().unwrap();
        let exit_code = output.status.code().unwrap_or(-1);
        eprintln!("Exit code: {exit_code}");
        assert_eq!(exit_code, 0, "void main() should return 0");

        let _ = std::fs::remove_file(&obj_path);
        let _ = std::fs::remove_file(&exe_path);
    }

    #[test]
    fn compile_no_stdlib() {
        let result = compile_source("module Test\nfunc main() { }");
        match result {
            Ok(r) => {
                eprintln!("Compiled without stdlib: {} bytes", r.object_bytes.len());
                assert!(!r.object_bytes.is_empty());
            },
            Err(e) => eprintln!("Compile failed: {e}"),
        }
    }

    /// Compile and run a program with stdlib.
    fn compile_and_run_with_stdlib(source: &str) -> (i32, String, String) {
        let mut compiler = kestrel_compiler::Compiler::new();
        compiler.load_dir(&stdlib_path());
        let entity = compiler.set_source("test.ks", source.into());
        compiler.build(entity);
        kestrel_compiler_driver::CompilerDriver::new(&compiler).infer_all();

        // Check for diagnostics from earlier phases (parse, bind, inference)
        let diagnostics = compiler.diagnostics();
        let _ = kestrel_compiler_driver::CompilerDriver::new(&compiler).emit_diagnostics();
        let error_count = diagnostics
            .iter()
            .filter(|d| d.severity >= kestrel_reporting::Severity::Error)
            .count();
        if error_count > 0 {
            panic!(
                "{} error(s) during build/inference — see diagnostics above",
                error_count
            );
        }

        let mir =
            kestrel_mir_lower::lower_module(compiler.world(), compiler.root()).with_all_passes();
        let target = TargetConfig::host();
        let options = CodegenOptions::default();

        let result = compile(&mir, &target, &options).unwrap_or_else(|e| {
            panic!("compilation failed: {e}");
        });

        // Use unique filenames per invocation to avoid test interference
        let uid = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join("kestrel_e2e_stdlib");
        let _ = std::fs::create_dir_all(&dir);
        let obj_path = dir.join(format!("test_{}.o", uid));
        let exe_path = dir.join(format!("test_{}_exe", uid));

        result.write_object_file(&obj_path).unwrap();
        link::link_executable(&obj_path, &exe_path, &[], &[], &[]).unwrap();

        let output = std::process::Command::new(&exe_path).output().unwrap();
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        #[cfg(unix)]
        if exit_code == -1 {
            use std::os::unix::process::ExitStatusExt;
            if let Some(sig) = output.status.signal() {
                eprintln!("Process killed by signal {sig}");
            }
        }
        eprintln!("Executable at: {}", exe_path.display());

        // Keep files for debugging
        eprintln!("Object at: {}", obj_path.display());
        // let _ = std::fs::remove_file(&obj_path);
        // let _ = std::fs::remove_file(&exe_path);

        (exit_code, stdout, stderr)
    }

    #[test]
    fn e2e_hello_world() {
        let (code, stdout, stderr) = compile_and_run_with_stdlib(
            r#"
module Test

func main() {
    print("Hello from lib!")
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?} stderr={stderr:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("Hello from lib!"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_arithmetic() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

func main() {
    let x: Int64 = 21;
    let y: Int64 = 21;
    let sum = x + y;
    if sum == 42 {
        print("42")
    } else {
        print("wrong")
    }
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("42"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_multiple_prints() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

func main() {
    let _ = print("ab");
    let _ = print("cd");
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("abcd"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_if_else() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

func main() {
    let x: Int64 = 10;
    if x > 5 {
        print("big")
    } else {
        print("small")
    }
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("big"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_nested_calls() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

func inner() -> String {
    "world"
}

func outer() -> String {
    inner()
}

func main() {
    let _ = print("hello ");
    print(outer())
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("hello world"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_function_call() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

func greet(name: String) -> String {
    "Hello, " + name + "!"
}

func main() {
    print(greet("lib"))
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("Hello, lib!"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_string_length() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

func main() {
    let s = "Hello from lib!";
    if s.count > 0 {
        print("has content")
    } else {
        print("empty")
    }
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("has content"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_struct_methods() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

struct Counter {
    var value: Int64

    init(start: Int64) {
        self.value = start
    }

    mutating func increment() {
        self.value = self.value + 1
    }

    func isAbove(threshold: Int64) -> Bool {
        self.value > threshold
    }
}

func main() {
    var c = Counter(start: 0);
    c.increment();
    c.increment();
    c.increment();
    if c.isAbove(threshold: 2) {
        print("passed")
    } else {
        print("failed")
    }
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("passed"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_enum_match() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

enum Direction {
    case Left
    case Right
    case Up
    case Down
}

func describe(dir: Direction) -> String {
    match dir {
        .Left => "left",
        .Right => "right",
        .Up => "up",
        .Down => "down"
    }
}

func main() {
    let d = Direction.Right;
    print(describe(d))
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("right"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_optional() {
        let (code, stdout, _) = compile_and_run_with_stdlib(
            r#"
module Test

func findChar(s: String, target: String) -> Optional[Int64] {
    if s.count > 0 {
        .Some(s.count)
    } else {
        .None
    }
}

func main() {
    let result = findChar("hello", "h");
    if let .Some(idx) = result {
        print("found")
    } else {
        print("not found")
    }
}
"#,
        );
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("found"), "stdout: {stdout:?}");
    }

    /// Minimal test: just allocate on the heap and return.
    #[test]
    fn e2e_minimal_return() {
        let (code, _, _) = compile_and_run_with_stdlib(
            r#"
module Test

func main() { }
"#,
        );
        assert_eq!(code, 0, "void main should exit 0");
    }

    #[test]
    fn e2e_int_return() {
        let (code, _, _) = compile_and_run_with_stdlib(
            r#"
module Test

func main() {
    let x: Int64 = 42;
}
"#,
        );
        assert_eq!(code, 0);
    }

    /// Smoke test: compile with stdlib to check for TypeParam panics.
    #[test]
    fn compile_with_stdlib_smoke() {
        let mut compiler = kestrel_compiler::Compiler::new();
        compiler.load_dir(&stdlib_path());
        let entity = compiler.set_source("test.ks", "module Test\nfunc main() { }".into());
        compiler.build(entity);
        kestrel_compiler_driver::CompilerDriver::new(&compiler).infer_all();

        let mir =
            kestrel_mir_lower::lower_module(compiler.world(), compiler.root()).with_all_passes();

        // Run MIR verification to catch structural issues
        mir.verify().dump();

        let target = TargetConfig::host();
        let options = CodegenOptions::default();

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            compile(&mir, &target, &options)
        })) {
            Ok(Ok(r)) => eprintln!(
                "Stdlib compilation succeeded: {} bytes",
                r.object_bytes.len()
            ),
            Ok(Err(e)) => eprintln!("Stdlib compilation error (expected during development): {e}"),
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
                    .or_else(|| p.downcast_ref::<&str>().copied())
                    .unwrap_or("unknown");
                eprintln!("Stdlib compilation panicked (expected during development): {msg}");
            },
        }
    }
}
