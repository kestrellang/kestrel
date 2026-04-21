//! File-based test harness using datatest-stable.
//!
//! Discovers all `.ks` files under `testdata/` and runs each as a separate test.
//! The test mode (diagnostics, mir, execution) is determined by the file header.

use std::path::Path;

use kestrel_test_suite2::TestCompiler;
use kestrel_test_suite2::annotation::{self, TestMode};
use kestrel_test_suite2::mir_snapshot;

fn run_ks_test(path: &Path) -> datatest_stable::Result<()> {
    let source = std::fs::read_to_string(path)?;
    let config = annotation::parse_test_config(&source);

    // Handle skip
    if let Some(reason) = &config.skip {
        eprintln!("SKIP: {} -- {}", path.display(), reason);
        return Ok(());
    }

    // Skip execution tests when KESTREL_SKIP_CODEGEN is set
    if config.test_mode == TestMode::Execution && std::env::var("KESTREL_SKIP_CODEGEN").is_ok() {
        eprintln!("SKIP (KESTREL_SKIP_CODEGEN): {}", path.display());
        return Ok(());
    }

    // Wrap in catch_unwind so internal compiler panics become test failures
    // instead of crashing the entire harness with SIGABRT
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

fn run_ks_test_inner(
    path: &Path,
    source: &str,
    config: &annotation::TestConfig,
) -> datatest_stable::Result<()> {
    let mut tc = if config.stdlib {
        TestCompiler::with_stdlib()
    } else {
        TestCompiler::new()
    };

    // Include extra source files relative to the test file's directory
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
            // Expect clean compilation before checking MIR
            tc.check_no_errors()?;
            let mir = tc.mir();
            mir_snapshot::check_mir_snapshot(
                path,
                &mir,
                config.mir_filter.as_deref(),
                config.mir_snapshot.as_deref(),
            )?;
        },

        TestMode::Execution => {
            // Expect clean compilation before running
            tc.check_no_errors()?;
            let result = tc
                .run()
                .map_err(|e| Into::<Box<dyn std::error::Error>>::into(e))?;

            // Check exit code
            let expected_exit = config.expect_exit.unwrap_or(0);
            if result.exit_code != expected_exit {
                return Err(format!(
                    "Expected exit code {}, got {}\nstdout: {}\nstderr: {}",
                    expected_exit, result.exit_code, result.stdout, result.stderr
                )
                .into());
            }

            // Check exact stdout
            if let Some(expected) = &config.expect_stdout {
                if result.stdout.trim() != expected.trim() {
                    return Err(format!(
                        "Stdout mismatch.\nExpected: {}\nActual:   {}",
                        expected.trim(),
                        result.stdout.trim()
                    )
                    .into());
                }
            }

            // Check stdout contains
            if let Some(needle) = &config.stdout_contains {
                if !result.stdout.contains(needle.as_str()) {
                    return Err(format!(
                        "Expected stdout to contain '{}'\nActual: {}",
                        needle, result.stdout
                    )
                    .into());
                }
            }
        },
    }

    Ok(())
}

datatest_stable::harness!(run_ks_test, "testdata", r"\.ks$");
