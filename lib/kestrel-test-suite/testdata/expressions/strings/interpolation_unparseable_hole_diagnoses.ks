// test: diagnostics
// stdlib: true
//
// #200 (BUG-66): an expression inside `\(...)` that fails to PARSE (here
// `n.0.0`, where the lexer eats `0.0` as a float) used to become an
// Error-typed value that slipped past inference and reached monomorphization
// as `appendInterpolation(type_args=[Error])` — an internal compiler error,
// with the real diagnostic swallowed. The build must instead fail cleanly
// with a diagnostic anchored at the hole.

module Test

func f() -> () {
    let n = ((1, 2), 3);
    let _ = "v=\(n.0.0)"; // ERROR: invalid expression in string interpolation
}
