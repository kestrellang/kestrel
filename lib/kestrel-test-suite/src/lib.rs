//! kestrel-test-suite — hybrid test framework for the lib compiler pipeline.
//!
//! Supports file-based `.ks` tests (auto-discovered via datatest-stable) and
//! a programmatic Rust API for complex/multi-file tests.
//!
//! # Stdlib Caching
//!
//! The stdlib is built once per process and cloned via `World::snapshot()`
//! per test. This avoids re-parsing/inferring 1500+ bodies for every test.

#[used]
#[unsafe(no_mangle)]
pub static BUILD_NONCE: u32 = 20;

pub mod annotation;
pub mod compiler;
pub mod diagnostic_matcher;
pub mod mir_snapshot;
pub mod runner;

pub use annotation::{AnnotationKind, TestConfig, TestMode};
pub use compiler::TestCompiler;
pub use diagnostic_matcher::TestDiagnostic;
pub use runner::RunResult;

use std::path::PathBuf;
use std::sync::OnceLock;

use kestrel_compiler_driver::CompilerDriver;
use kestrel_compiler::Compiler;

/// Cached stdlib compiler state. Built once, cloned per test.
struct StdlibCache {
    compiler: Compiler,
}

// Safety: StdlibCache is initialized once (via OnceLock) and then only accessed
// via world().snapshot() which clones all data into a fresh, independent World.
// No concurrent mutation occurs — the cached Compiler is read-only after init.
unsafe impl Send for StdlibCache {}
unsafe impl Sync for StdlibCache {}

static STDLIB_CACHE: OnceLock<StdlibCache> = OnceLock::new();

/// Get or initialize the cached stdlib compiler.
fn stdlib_cache() -> &'static StdlibCache {
    STDLIB_CACHE.get_or_init(|| {
        let mut compiler = Compiler::new();
        let std_path = find_stdlib_path();
        compiler.load_dir(&std_path);
        CompilerDriver::new(&compiler).infer_all();
        StdlibCache { compiler }
    })
}

/// Create a test compiler with or without stdlib pre-loaded.
pub fn test_compiler(with_stdlib: bool) -> Compiler {
    if with_stdlib {
        let cache = stdlib_cache();
        let snapshot = cache.compiler.world().snapshot();
        Compiler::from_snapshot(
            snapshot,
            cache.compiler.root(),
            cache.compiler.files().clone(),
        )
    } else {
        Compiler::new()
    }
}

/// Locate the stdlib directory.
///
/// Searches: KESTREL_STD env var, then relative to CARGO_MANIFEST_DIR.
fn find_stdlib_path() -> PathBuf {
    if let Ok(path) = std::env::var("KESTREL_STD") {
        return PathBuf::from(path);
    }
    // lib/kestrel-test-suite/ -> lib/ -> project root -> lang/std
    let manifest = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest)
        .parent()
        .unwrap() // lib/
        .parent()
        .unwrap(); // project root
    project_root.join("lang/std")
}

// trigger rebuild after stdlib substring refactor
