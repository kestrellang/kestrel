//! Diagnostics are deterministic: the same program gives the same set, in
//! source order, on every run (bidi design §10, P0).
//!
//! Before: the type checker iterated std `HashMap`s, which are seeded per
//! map, so which expression claimed a "could not infer type" — and whether
//! one or two of them came out — varied from run to run; and
//! `Compiler::diagnostics()` returned bodies in accumulator map order.
//! Building fresh compilers in one process exercises both, since every new
//! std map gets a new seed.

use kestrel_compiler::Compiler;
use kestrel_compiler_driver::CompilerDriver;

/// `(line start offset, message)` for every diagnostic, in returned order.
fn diagnostics_of(src: &str) -> Vec<(usize, String)> {
    let mut compiler = Compiler::new();
    let file = compiler.set_source("/tmp/determinism.ks", src.to_string());
    compiler.build(file);
    let _ = CompilerDriver::new(&compiler).infer_all();
    compiler
        .diagnostics()
        .iter()
        .map(|d| {
            let start = d.labels.first().map_or(usize::MAX, |l| l.range.start);
            (start, d.message.clone())
        })
        .collect()
}

#[test]
fn the_same_program_gives_the_same_diagnostics_every_time() {
    // From `expressions/closures/cannot_infer_it_type_without_context.ks`,
    // which produced one or two "could not infer type" errors run to run.
    let src = "module Main\n\nfunc test() {\n    let f = { it };\n}\n";
    let first = diagnostics_of(src);
    assert!(!first.is_empty(), "expected a 'could not infer' error");
    for run in 1..12 {
        assert_eq!(diagnostics_of(src), first, "run {run} differs from run 0");
    }
}

#[test]
fn diagnostics_come_back_in_source_order() {
    // One error per body, in six bodies: accumulator order is per query, so
    // an unsorted result is almost never accidentally in order.
    let src: String = std::iter::once("module Main\n".to_string())
        .chain((0..6).map(|i| format!("func f{i}() {{ let x: lang.i64 = undefined{i}; }}\n")))
        .collect();
    let diags = diagnostics_of(&src);
    assert!(diags.len() >= 6, "expected one error per body: {diags:?}");
    let starts: Vec<usize> = diags.iter().map(|(s, _)| *s).collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted, "diagnostics out of source order: {diags:?}");
}

/// Message and label text of every diagnostic.
fn rendered_text(src: &str) -> Vec<String> {
    let mut compiler = Compiler::new();
    let file = compiler.set_source("/tmp/placeholders.ks", src.to_string());
    compiler.build(file);
    let _ = CompilerDriver::new(&compiler).infer_all();
    compiler
        .diagnostics()
        .iter()
        .map(|d| {
            let labels: Vec<&str> = d.labels.iter().map(|l| l.message.as_str()).collect();
            format!("{} | {}", d.message, labels.join(" | "))
        })
        .collect()
}

#[test]
fn messages_never_print_the_error_placeholder() {
    // `closure_arity_mismatch_too_few.ks`: its only error used to read
    // "expected (i64, i64) -> i64 got (Error) -> Error". The poisoned parts
    // print as `_`; the error itself must still be reported — it is the
    // only one (the `Error` came from `poison`, not from another report).
    let src = "module Main\n\nfunc test() -> (lang.i64, lang.i64) -> lang.i64 {\n    { (x) in x }\n}\n";
    let text = rendered_text(src);
    assert_eq!(text.len(), 1, "expected exactly the arity mismatch: {text:?}");
    assert!(
        !text[0].contains("Error") && !text[0].contains('?'),
        "placeholder leaked into the message: {text:?}"
    );
    assert!(text[0].contains("(_) -> _"), "unexpected rendering: {text:?}");
}
