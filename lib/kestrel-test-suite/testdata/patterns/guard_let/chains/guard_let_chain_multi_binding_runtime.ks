// test: execution
// stdlib: false

// Regression for #126: a `guard` with several comma-chained `let` bindings
// must CPS-lower so the bindings flow into the continuation. The old lowering
// routed multi-condition guards through the boolean-AND fallback, which
// evaluated each pattern only for its truth value and dropped the binding —
// the continuation then referenced binding locals that were never threaded
// into its block, tripping OSSA verification ("used but never defined").
// Single source of truth: lower_guard_cps in kestrel-hir-lower/src/expr.rs.
module Main

enum Option[T] {
    case Some(T)
    case None
}

// Constructors: the typed local gives `.Some`/`.None` an explicit expected
// type so the generic case resolves without the stdlib literal-default path.
func mkSome(v: lang.i64) -> Option[lang.i64] {
    let r: Option[lang.i64] = .Some(v);
    r
}

func mkNone() -> Option[lang.i64] {
    let r: Option[lang.i64] = .None;
    r
}

// Two comma-chained patterns; both bindings must reach the tail expression.
func sum(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = a, let .Some(y) = b else {
        return 0
    }
    lang.i64_add(x, y)
}

// An earlier binding must be visible to a later boolean condition AND the tail.
// Returns the sentinel 999 when the ordering condition fails.
func diffIfOrdered(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = a, let .Some(y) = b, lang.i64_signed_lt(x, y) else {
        return 999
    }
    lang.i64_sub(y, x)
}

// Three comma-chained patterns nest three levels deep.
func sum3(a: Option[lang.i64], b: Option[lang.i64], c: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = a, let .Some(y) = b, let .Some(z) = c else {
        return 0
    }
    lang.i64_add(lang.i64_add(x, y), z)
}

@main
func main() -> lang.i64 {
    // Success path: both bindings flow → 3 + 4 = 7.
    if lang.i64_ne(sum(mkSome(3), mkSome(4)), 7) { return 1; }
    // Failure path: second pattern is None → else runs, returns 0.
    if lang.i64_ne(sum(mkSome(3), mkNone()), 0) { return 2; }
    // Binding visible to later boolean condition, condition holds → 5 - 2 = 3.
    if lang.i64_ne(diffIfOrdered(mkSome(2), mkSome(5)), 3) { return 3; }
    // Same, but the trailing boolean condition fails → else runs, returns 999.
    if lang.i64_ne(diffIfOrdered(mkSome(9), mkSome(1)), 999) { return 4; }
    // Three-pattern chain → 1 + 2 + 3 = 6.
    if lang.i64_ne(sum3(mkSome(1), mkSome(2), mkSome(3)), 6) { return 5; }
    // Three-pattern chain with a middle None → else runs, returns 0.
    if lang.i64_ne(sum3(mkSome(1), mkNone(), mkSome(3)), 0) { return 6; }
    0
}
