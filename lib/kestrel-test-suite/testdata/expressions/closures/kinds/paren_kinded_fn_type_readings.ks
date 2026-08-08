// test: execution
// stdlib: true
// expect-exit: 0

// Plan D2 (docs/plans/closure-kinds/closure-kinds-plan.md), the `elem` parser
// hazard: the per-param type parser greedily consumes `Mutating` via `or_not()`
// with no backtracking (ty/mod.rs:180-188), so today `(mutating () -> ())`
// silently parses as a GROUPING that drops the marker. `elem` must attempt the
// full `ty` (which now includes kind prefixes) FIRST and only then fall back to
// `Mutating? + ty`.
//
// Two readings are pinned here by RUNTIME behavior:
//
//   1. `(mutating () -> ())` used as a type annotation is the mutating-KIND
//      function type — parenthesizing must not drop the kind. Write-back
//      through the captures is the proof: a normal-kind body assigning to a
//      capture is E603, so this only compiles if the kind survived the parens.
//   2. `(escaping () -> Int64) -> Int64` is a function type whose PARAM is an
//      escaping-kind closure. `callEscaping` has exactly that type; if the
//      param's kind were dropped the annotation would read
//      `(() -> Int64) -> Int64` and the assignment would be a kind mismatch.
//
// The third reading, `(mutating (T) -> R) -> U`, cannot be exercised by a call
// in v1 (a mutating-kind parameter needs a `mutating` convention, and
// conventions inside function types are out of scope per D2) — it is pinned in
// the sibling paren_kinded_fn_type_param_kind_not_dropped.ks.
module Test

import std.numeric.(Int64)

func callEscaping(f: escaping () -> Int64) -> Int64 { f() }

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }
}

@main
func main() -> lang.i64 {
    // (1) parenthesized mutating-kind annotation — the grouping keeps the kind
    var total = 0;
    var bump: (mutating () -> ()) = { total = total + 5; };
    bump();
    bump();
    if total != 10 { return 1 }

    // (2) `(escaping () -> Int64) -> Int64` — the PARAM is escaping-kind
    let g: (escaping () -> Int64) -> Int64 = callEscaping;
    let next = makeCounter(0);
    if g(next) != 1 { return 2 }
    if g(next) != 2 { return 3 }   // shared environment survived the hand-off

    0
}
