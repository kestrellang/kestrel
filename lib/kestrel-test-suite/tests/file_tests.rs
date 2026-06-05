//! File-based test harness.
//!
//! Discovers all `.ks` files under `testdata/` and runs each as a separate
//! libtest trial. The test mode (diagnostics, mir, execution) is determined by
//! the file header.
//!
//! In addition to the standard libtest-mimic flags this binary accepts a
//! `--names-file PATH` flag (or `--names-file=PATH`). When supplied, only
//! trials whose fully-qualified name appears in the file are discovered —
//! exactly one name per line, matching the format libtest prints (e.g.
//! `run_ks_test::codegen/arithmetic/add.ks`). External drivers like the
//! `triage` tool use this to batch many tests into a single subprocess so
//! expensive one-time setup (e.g. stdlib parsing/inference) is amortized
//! across the batch.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use libtest_mimic::{Arguments, Trial};
use walkdir::WalkDir;

use kestrel_test_suite::TestCompiler;
use kestrel_test_suite::annotation::{self, TestMode};
use kestrel_test_suite::mir_snapshot;

// Matches datatest-stable's looser bound — we `format!` the error into a
// `String` before returning it to the libtest thread, so Send/Sync on the
// boxed error itself isn't required.
type TestResult = Result<(), Box<dyn std::error::Error>>;

const HARNESS_NAME: &str = "run_ks_test";
const TEST_ROOT: &str = "testdata";
const TEST_EXT: &str = "ks";

fn main() -> ExitCode {
    let raw_args: Vec<String> = env::args().collect();
    let (names_filter, forwarded) = extract_names_filter(raw_args);
    let args = Arguments::from_iter(forwarded);
    let trials = discover_trials(names_filter.as_ref());
    libtest_mimic::run(&args, trials).exit_code()
}

/// Pulls our custom `--names-file` flag out of argv and loads the requested
/// set of libtest names. Everything else is forwarded to libtest-mimic.
fn extract_names_filter(args: Vec<String>) -> (Option<HashSet<String>>, Vec<String>) {
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut names_path: Option<String> = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--names-file" {
            names_path = it.next();
            if names_path.is_none() {
                panic!("--names-file requires a path argument");
            }
        } else if let Some(v) = a.strip_prefix("--names-file=") {
            names_path = Some(v.to_string());
        } else {
            out.push(a);
        }
    }
    let names = names_path.map(|path| {
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read --names-file '{path}': {e}"));
        contents
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect::<HashSet<_>>()
    });
    (names, out)
}

fn discover_trials(filter: Option<&HashSet<String>>) -> Vec<Trial> {
    let root = Path::new(TEST_ROOT);
    let mut trials = Vec::new();
    for entry in WalkDir::new(root).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(TEST_EXT) {
            continue;
        }
        // Skip hidden files (match datatest-stable's convention).
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }

        let rel = path
            .strip_prefix(root)
            .expect("walkdir entry must be descendant of root");
        // Normalize to forward slashes so names are stable across platforms
        // and round-trip with triage's stored names.
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let name = format!("{HARNESS_NAME}::{rel_str}");

        if let Some(set) = filter
            && !set.contains(&name)
        {
            continue;
        }

        let test_path: PathBuf = path.to_path_buf();
        trials.push(Trial::test(name, move || {
            run_ks_test(&test_path).map_err(|e| format!("{:?}", e).into())
        }));
    }
    trials.sort_by(|a, b| a.name().cmp(b.name()));
    trials
}

fn run_ks_test(path: &Path) -> TestResult {
    let source = std::fs::read_to_string(path)?;
    let config = annotation::parse_test_config(&source);

    if let Some(reason) = &config.skip {
        eprintln!("SKIP: {} -- {}", path.display(), reason);
        return Ok(());
    }

    if config.test_mode == TestMode::Execution && std::env::var("KESTREL_SKIP_CODEGEN").is_ok() {
        eprintln!("SKIP (KESTREL_SKIP_CODEGEN): {}", path.display());
        return Ok(());
    }

    // Wrap in catch_unwind so internal compiler panics become test failures
    // instead of crashing the entire harness with SIGABRT. This matters even
    // more in batched mode — a single panic must not take down the whole batch.
    let path_owned = path.to_owned();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_ks_test_inner(&path_owned, &source, &config)
    }));

    match result {
        Ok(inner) => inner,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };
            Err(format!("PANIC: {}", msg).into())
        },
    }
}

fn run_ks_test_inner(path: &Path, source: &str, config: &annotation::TestConfig) -> TestResult {
    let mut tc = if config.stdlib {
        TestCompiler::with_stdlib()
    } else {
        TestCompiler::new()
    };
    // Execution tests build a binary, so they require a `@main` (E618). A
    // diagnostics test can opt in with `// executable: true` to exercise it.
    tc.set_executable(config.test_mode == TestMode::Execution || config.executable);

    for include_path in &config.include {
        let include_file = path.parent().unwrap().join(include_path);
        let include_source = std::fs::read_to_string(&include_file).unwrap_or_else(|e| {
            panic!("failed to read include '{}': {}", include_file.display(), e)
        });
        tc.add_source(&include_file.to_string_lossy(), &include_source);
    }

    let file_path = path.to_string_lossy();
    let entity = tc.add_source(&file_path, source);
    let file_id = entity.index();

    match config.test_mode {
        TestMode::Diagnostics => {
            let annotations = annotation::parse_annotations(source);
            tc.check_annotations(&annotations, file_id)?;
        },

        TestMode::Mir => {
            tc.check_no_errors()?;
            let mir = tc.mir().map_err(Into::<Box<dyn std::error::Error>>::into)?;
            mir_snapshot::check_mir_snapshot(
                path,
                &mir,
                config.mir_filter.as_deref(),
                config.mir_snapshot.as_deref(),
            )?;
        },

        TestMode::Execution => {
            tc.check_no_errors()?;
            let result = tc.run().map_err(Into::<Box<dyn std::error::Error>>::into)?;

            let expected_exit = config.expect_exit.unwrap_or(0);
            if result.exit_code != expected_exit {
                return Err(format!(
                    "Expected exit code {}, got {}\nstdout: {}\nstderr: {}",
                    expected_exit, result.exit_code, result.stdout, result.stderr
                )
                .into());
            }

            if let Some(expected) = &config.expect_stdout
                && result.stdout.trim() != expected.trim()
            {
                return Err(format!(
                    "Stdout mismatch.\nExpected: {}\nActual:   {}",
                    expected.trim(),
                    result.stdout.trim()
                )
                .into());
            }

            if let Some(needle) = &config.stdout_contains
                && !result.stdout.contains(needle.as_str())
            {
                return Err(format!(
                    "Expected stdout to contain '{}'\nActual: {}",
                    needle, result.stdout
                )
                .into());
            }
        },
    }

    Ok(())
}
