// test: execution
// stdlib: false

// Regression for #126's if-let twin: an `if let` with several comma-chained
// `let` bindings must thread the bindings into the then-branch. The old
// lowering only special-cased a single binding and routed multi-condition
// if-lets through the boolean-AND path, which dropped the bindings and tripped
// OSSA verification ("used but never defined"). Shared lowering: the condition
// chain in kestrel-hir-lower (lower_condition_chain / lower_if).
module Main

enum Option[T] {
    case Some(T)
    case None
}

func mkSome(v: lang.i64) -> Option[lang.i64] {
    let r: Option[lang.i64] = .Some(v);
    r
}

func mkNone() -> Option[lang.i64] {
    let r: Option[lang.i64] = .None;
    r
}

// Two comma-chained patterns; both bindings must reach the then-branch.
func sum(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a, let .Some(y) = b {
        lang.i64_add(x, y)
    } else {
        0
    }
}

// Earlier binding visible to a later boolean condition AND the then-branch.
func diffIfOrdered(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a, let .Some(y) = b, lang.i64_signed_lt(x, y) {
        lang.i64_sub(y, x)
    } else {
        999
    }
}

@main
func main() -> lang.i64 {
    // Both bindings flow → 3 + 4 = 7.
    if lang.i64_ne(sum(mkSome(3), mkSome(4)), 7) { return 1; }
    // Second pattern is None → else branch → 0.
    if lang.i64_ne(sum(mkSome(3), mkNone()), 0) { return 2; }
    // Binding visible to the later boolean condition, condition holds → 3.
    if lang.i64_ne(diffIfOrdered(mkSome(2), mkSome(5)), 3) { return 3; }
    // Trailing boolean condition fails → else → 999.
    if lang.i64_ne(diffIfOrdered(mkSome(9), mkSome(1)), 999) { return 4; }
    0
}
