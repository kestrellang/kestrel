//! Build a single .ks test file into a standalone executable for debugging.
//!
//! Usage:
//!   cargo run --example build_test --release -- <path-to-ks> <output-binary>
//!
//! This compiles the given .ks test file (with stdlib) and writes a native
//! binary to the given output path. Does NOT run it — just builds it so you
//! can debug under lldb.

use std::path::Path;

use kestrel_test_suite2::TestCompiler;

fn main() {
    let mut args = std::env::args().skip(1);
    let ks_path = args.next().expect("usage: build_test <ks-path> <output-binary>");
    let out_path = args.next().expect("usage: build_test <ks-path> <output-binary>");

    let source = std::fs::read_to_string(&ks_path).expect("failed to read ks file");

    let mut tc = TestCompiler::with_stdlib();
    let _entity = tc.add_source(&ks_path, &source);

    // Force inference + diagnostics check
    let diags = tc.all_diagnostics();
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.severity, kestrel_test_suite2::diagnostic_matcher::TestSeverity::Error))
        .collect();
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ERROR: {:?}", e);
        }
        std::process::exit(1);
    }

    let options = kestrel_codegen2_cranelift::CodegenOptions::default();
    tc.compiler()
        .compile_and_link(Path::new(&out_path), &options)
        .expect("compile_and_link failed");

    eprintln!("built: {}", out_path);
}
