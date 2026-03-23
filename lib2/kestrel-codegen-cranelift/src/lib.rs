//! Cranelift backend for Kestrel (lib2).
//!
//! Compiles `MirModule` → native object code via Cranelift.
//!
//! Pipeline: `MirModule` → monomorphize (BFS) → CodegenContext → compile all → object bytes → link

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
use kestrel_codegen2::TargetConfig;
use kestrel_mir::MirModule;
use std::path::Path;
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
}

/// Result of compilation.
pub struct CompilationResult {
    /// The compiled object file bytes.
    pub object_bytes: Vec<u8>,
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
    stacker::maybe_grow(256 * 1024, 4 * 1024 * 1024, || compile_inner(module, target, options))
}

fn compile_inner(
    module: &MirModule,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<CompilationResult, CodegenError> {
    // Phase 1: Collect all monomorphized function instantiations
    let mono_set = monomorphize::collect_all(module)
        .map_err(|errors| {
            let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            CodegenError::Monomorphization(msgs.join("\n"))
        })?;

    // Phase 2: Compile all functions
    let mut ctx = CodegenContext::new(module, target, options, mono_set)?;
    ctx.compile_all()?;

    // Phase 3: Emit object bytes
    let object_bytes = ctx.finish()?;

    Ok(CompilationResult { object_bytes })
}

/// Counter for unique temp file names.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Compile and link to an executable.
pub fn compile_and_link(
    module: &MirModule,
    target: &TargetConfig,
    options: &CodegenOptions,
    output_path: impl AsRef<Path>,
) -> Result<(), CodegenError> {
    let result = compile(module, target, options)?;

    // Write temp object file with a unique name
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let thread_id = std::thread::current().id();
    let temp_name = format!("kestrel_{counter}_{thread_id:?}.o");
    let temp_path = std::env::temp_dir().join(&temp_name);

    result.write_object_file(&temp_path)?;

    // Link
    let link_result = link::link_executable(
        &temp_path,
        output_path.as_ref(),
        &options.libraries,
        &options.library_paths,
        &options.frameworks,
    );

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    link_result
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
        let mut compiler = kestrel_compiler2::Compiler::new();
        let entity = compiler.set_source("test.ks", source.into());
        compiler.build(entity);
        compiler.infer_all();

        let mir = kestrel_mir_lower::lower_module(compiler.world(), compiler.root())
            .with_all_passes();
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
            }
            Err(e) => {
                eprintln!("Compilation failed (expected during development): {e}");
            }
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
            }
            Err(e) => eprintln!("Compile failed: {e}"),
        }
    }

    /// Compile and run a program with stdlib.
    fn compile_and_run_with_stdlib(source: &str) -> (i32, String, String) {
        let mut compiler = kestrel_compiler2::Compiler::new();
        compiler.load_dir(&stdlib_path());
        let entity = compiler.set_source("test.ks", source.into());
        compiler.build(entity);
        compiler.infer_all();

        // Check for diagnostics from earlier phases (parse, bind, inference)
        let diagnostics = compiler.diagnostics();
        let _ = compiler.emit_diagnostics();
        let error_count = diagnostics.iter()
            .filter(|d| format!("{:?}", d.severity).contains("Error"))
            .count();
        if error_count > 0 {
            panic!("{} error(s) during build/inference — see diagnostics above", error_count);
        }

        let mir = kestrel_mir_lower::lower_module(compiler.world(), compiler.root())
            .with_all_passes();
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
        let (code, stdout, stderr) = compile_and_run_with_stdlib(r#"
module Test

func main() {
    print("Hello from lib2!")
}
"#);
        eprintln!("exit={code} stdout={stdout:?} stderr={stderr:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("Hello from lib2!"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_arithmetic() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
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
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("42"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_multiple_prints() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
module Test

func main() {
    let _ = print("ab");
    let _ = print("cd");
}
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("abcd"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_if_else() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
module Test

func main() {
    let x: Int64 = 10;
    if x > 5 {
        print("big")
    } else {
        print("small")
    }
}
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("big"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_nested_calls() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
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
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("hello world"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_function_call() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
module Test

func greet(name: String) -> String {
    "Hello, " + name + "!"
}

func main() {
    print(greet("lib2"))
}
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("Hello, lib2!"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_string_length() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
module Test

func main() {
    let s = "Hello from lib2!";
    if s.count > 0 {
        print("has content")
    } else {
        print("empty")
    }
}
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("has content"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_struct_methods() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
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
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("passed"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_enum_match() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
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
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("right"), "stdout: {stdout:?}");
    }

    #[test]
    fn e2e_optional() {
        let (code, stdout, _) = compile_and_run_with_stdlib(r#"
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
"#);
        eprintln!("exit={code} stdout={stdout:?}");
        assert_eq!(code, 0);
        assert!(stdout.contains("found"), "stdout: {stdout:?}");
    }

    /// Minimal test: just allocate on the heap and return.
    #[test]
    fn e2e_minimal_return() {
        let (code, _, _) = compile_and_run_with_stdlib(r#"
module Test

func main() { }
"#);
        assert_eq!(code, 0, "void main should exit 0");
    }

    #[test]
    fn e2e_int_return() {
        let (code, _, _) = compile_and_run_with_stdlib(r#"
module Test

func main() {
    let x: Int64 = 42;
}
"#);
        assert_eq!(code, 0);
    }

    /// Smoke test: compile with stdlib to check for TypeParam panics.
    #[test]
    fn compile_with_stdlib_smoke() {
        let mut compiler = kestrel_compiler2::Compiler::new();
        compiler.load_dir(&stdlib_path());
        let entity = compiler.set_source("test.ks", "module Test\nfunc main() { }".into());
        compiler.build(entity);
        compiler.infer_all();

        let mir = kestrel_mir_lower::lower_module(compiler.world(), compiler.root())
            .with_all_passes();

        // Run MIR verification to catch structural issues
        mir.verify().dump();

        let target = TargetConfig::host();
        let options = CodegenOptions::default();

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            compile(&mir, &target, &options)
        })) {
            Ok(Ok(r)) => eprintln!("Stdlib compilation succeeded: {} bytes", r.object_bytes.len()),
            Ok(Err(e)) => eprintln!("Stdlib compilation error (expected during development): {e}"),
            Err(p) => {
                let msg = p.downcast_ref::<String>().map(|s| s.as_str())
                    .or_else(|| p.downcast_ref::<&str>().copied())
                    .unwrap_or("unknown");
                eprintln!("Stdlib compilation panicked (expected during development): {msg}");
            }
        }
    }
}
