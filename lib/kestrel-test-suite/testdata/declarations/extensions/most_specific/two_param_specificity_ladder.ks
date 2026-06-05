// test: execution
// stdlib: false
// expect-exit: 0

// Specificity ordering across two type parameters. Three overlapping
// extensions form a ladder: fully generic (1), half specialized on the second
// param (2), fully specialized (3). Each concrete instantiation must resolve to
// the most specific applicable extension. Execution counterpart of the
// diagnostics-only `more_specialized_wins.ks`. Any mis-selection returns a
// nonzero code identifying which instantiation picked the wrong rung.
module Main

struct Pair[T, U] { var a: T; var b: U }

extend Pair[T, U]               { func tag() -> lang.i64 { 1 } }
extend Pair[T, lang.i64]        { func tag() -> lang.i64 { 2 } }
extend Pair[lang.i64, lang.i64] { func tag() -> lang.i64 { 3 } }

@main
func main() -> lang.i64 {
    // (str, str): only the fully generic extension applies -> 1
    if lang.i64_ne(Pair[lang.str, lang.str](a: "x", b: "y").tag(), 1) { return 1; }
    // (str, i64): half specialized is the most specific applicable -> 2
    if lang.i64_ne(Pair[lang.str, lang.i64](a: "x", b: 0).tag(), 2) { return 2; }
    // (i64, i64): fully specialized wins over both -> 3
    if lang.i64_ne(Pair[lang.i64, lang.i64](a: 0, b: 0).tag(), 3) { return 3; }
    0
}
