// test: diagnostics
// stdlib: false

// A destructured parameter has no source name, so the AST builder synthesizes
// one — and E613 prints it verbatim. The synthetic name must be positional
// within the parameter list (`_param_0` for the first one), not a function of
// how many declarations the process has already built.
//
// It used to come from a process-lifetime `static AtomicU32` (F43b): the LSP,
// which holds one long-lived `Compiler`, reported `_param_0`, then `_param_7`,
// then `_param_23` for the same unedited file, and the test harness — one
// process, parallel threads — raced on the value. This fixture pins the name;
// `builders/params.rs`'s unit tests pin the no-carryover property.

module Main

func plot(a: lang.i64 = 0, (x, y): (lang.i64, lang.i64)) -> lang.i64 { // ERROR: required parameter '_param_0' cannot follow parameter 'a' which has a default value
    x
}
