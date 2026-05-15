// test: diagnostics
// stdlib: true

// Regression: matching a scrutinee whose type-checker produced an error
// must not panic in exhaustiveness analysis. Before the fix, this triggered
// `PatternMatrix::push` with a row/column arity mismatch (the `??` left a
// sentinel entity behind in `resolve_implicit_variant`).
module Test

func main() -> lang.i64 {
    let opt: std.result.Optional[std.numeric.Int64] = .None;
    let r = opt ?? null; // ERROR: type mismatch
    match r { // ERROR: non-exhaustive
        .Some(_) => 0,
        .None => 1
    }
}
