//! Build one or more .ks files into a standalone executable for debugging.
//!
//! Usage:
//!   cargo run --example build_test --release -- \
//!       -o <output-binary> [-l <lib-or-:path.o>]... <ks-path>...
//!
//! Compiles the given .ks files (with stdlib) and writes a native binary.
//! `-l foo` → `-lfoo`; `-l :/abs/path.o` → linker is given the literal path.

use std::path::Path;

use kestrel_test_suite::TestCompiler;

fn main() {
    let mut out_path: Option<String> = None;
    let mut libraries: Vec<String> = Vec::new();
    let mut ks_paths: Vec<String> = Vec::new();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                out_path = Some(args[i].clone());
            },
            "-l" => {
                i += 1;
                libraries.push(args[i].clone());
            },
            p => ks_paths.push(p.to_string()),
        }
        i += 1;
    }

    let out_path = out_path.unwrap_or_else(|| {
        eprintln!("usage: build_test -o <output-binary> [-l lib]... <ks-path>...");
        std::process::exit(2);
    });
    if ks_paths.is_empty() {
        eprintln!("usage: build_test -o <output-binary> [-l lib]... <ks-path>...");
        std::process::exit(2);
    }

    let mut tc = TestCompiler::with_stdlib();
    for ks_path in &ks_paths {
        let source = std::fs::read_to_string(ks_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", ks_path, e));
        tc.add_source(ks_path, &source);
    }

    let diags = tc.all_diagnostics();
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| {
            matches!(
                d.severity,
                kestrel_test_suite::diagnostic_matcher::TestSeverity::Error
            )
        })
        .collect();
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ERROR: {:?}", e);
        }
        std::process::exit(1);
    }

    let options = kestrel_codegen_cranelift_2::CodegenOptions {
        libraries,
        ..Default::default()
    };
    tc.compiler()
        .compile_and_link2(Path::new(&out_path), &options)
        .expect("compile_and_link2 failed");

    eprintln!("built: {}", out_path);
}
