//! kestrel-test-suite — hybrid test framework for the lib compiler pipeline.
//
// (cache-bust: 2026-04-27d)
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
pub static BUILD_NONCE: u32 = 24;

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

use kestrel_compiler::Compiler;
use kestrel_compiler_driver::CompilerDriver;

/// Cached stdlib compiler state. Built once, cloned per test.
struct StdlibCache {
    compiler: Compiler,
    /// Rendered stdlib-side errors ("path:line: message") found while building
    /// the cache. Diagnostics are emitted into the CACHE compiler's sink at
    /// first query execution; per-test compilers get memoized cache hits that
    /// never re-emit, so without this record a stdlib type error is invisible
    /// to every test and only surfaces as a mysterious downstream mono ICE.
    errors: Vec<String>,
}

// UNSOUND — audit finding F42, kept deliberately because no fix is free.
//
// The retired justification was: "initialized once via OnceLock, then only
// accessed via world().snapshot(), which clones all data into a fresh,
// independent World. No concurrent mutation occurs — the cached Compiler is
// read-only after init." That reasoning misses where the mutation is.
// `snapshot()` clones the query memos, one of which is `Parse`, whose
// `ParseResult` holds a rowan CST behind a NON-ATOMIC refcount. Cloning it
// mutates a counter shared with the cache, so two threads snapshotting (or one
// snapshotting while another drops its snapshot) race on that counter. The
// object graph is shared, not copied.
//
// A Mutex around `snapshot()` does NOT fix it: the returned snapshot outlives
// the lock, and dropping it decrements the same shared counters. Locking
// narrows the window; it never closes it. The clean fix — `Send + Sync` on
// `QueryFn::Output` — was probed and fails on `ParseResult`, since rowan's
// `SyntaxNode` is inherently `!Send`/`!Sync`.
//
// Live, not theoretical: libtest_mimic runs trials as parallel threads in one
// process and triage batches many tests per subprocess. It is why the triage
// default `-j` is conservative. Fixing it means choosing a cost — isolated
// processes, re-parsing per test, single-threading, or a thread-local cache.
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
        // Render error-severity diagnostics NOW, against the cache compiler's
        // own world — per-test compilers can't recover them (see field doc).
        let errors = render_stdlib_errors(&compiler);
        // Clear the changed set so cached queries survive snapshot.
        // Queries whose deps point to unchanged stdlib entities will
        // be verified as cache hits instead of re-executing.
        compiler.begin_revision();
        StdlibCache { compiler, errors }
    })
}

/// Error-severity stdlib diagnostics rendered as "path:line: message" strings.
/// Empty when the stdlib is clean (the normal case). Exposed so per-test
/// assertions (`TestCompiler::check_no_errors`) can surface a broken stdlib
/// instead of letting it masquerade as unrelated mono/codegen failures.
pub fn stdlib_errors() -> &'static [String] {
    &stdlib_cache().errors
}

fn render_stdlib_errors(compiler: &Compiler) -> Vec<String> {
    use crate::diagnostic_matcher::{TestSeverity, from_codespan_diagnostics};
    let world = compiler.world();
    let mut sources = Vec::new();
    let mut names: std::collections::HashMap<usize, String> = Default::default();
    for (path, &entity) in compiler.files() {
        if let Some(src) = world.get::<kestrel_compiler::SourceText>(entity) {
            sources.push((entity.index(), src.0.clone()));
            // Short path: last two components ("numeric/random.ks").
            let short = std::path::Path::new(path)
                .iter()
                .rev()
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<std::path::PathBuf>()
                .to_string_lossy()
                .to_string();
            names.insert(entity.index(), short);
        }
    }
    from_codespan_diagnostics(&compiler.diagnostics(), &sources)
        .into_iter()
        .filter(|d| d.severity == TestSeverity::Error)
        .map(|d| {
            let file = names.get(&d.file_id).map(|s| s.as_str()).unwrap_or("<std>");
            format!("{}:{}: {}", file, d.line, d.message)
        })
        .collect()
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
/// Shares `kestrel_compiler::stdlib_path` with the CLI and the LSP. This used
/// to take `KESTREL_STD` unconditionally: a stale value loaded zero stdlib
/// files, and every test then failed on unrelated "unknown type" errors.
/// Panicking here is the right shape for a test harness — a test run with no
/// stdlib is not a result worth recording.
fn find_stdlib_path() -> PathBuf {
    kestrel_compiler::stdlib_path::default_std_path()
        .unwrap_or_else(|e| panic!("test suite could not locate the stdlib:\n{e}"))
}

// trigger rebuild after stdlib substring refactor
