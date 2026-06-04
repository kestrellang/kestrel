// test: diagnostics
// stdlib: true

// KNOWN BUG (3-i) — characterization test, NOT desired behavior.
//
// Value promotion (`T` -> `Result[T,E]` / `Optional[T]`) fires for bare values
// (literals, locals, params) but NOT for a COMPUTED-EXPRESSION tail/return
// (e.g. `y + 1`, a call result). In `solve_coerce`, the eager `unify(from, to)`
// binds the still-unresolved expression-result tyvar to the target *before* the
// expression's own constraint resolves it, so the FromValue promotion never
// applies and a spurious type mismatch (E100) is reported instead of wrapping
// in `.Ok` / `.Some`. (Bare values work because their tyvar is already bound.)
//
// This test asserts the CURRENT (buggy) type-mismatch so the suite stays green
// until the bug is fixed. WHEN 3-i IS FIXED this test FAILS ("missing expected
// error") — that is the signal to remove the annotation and assert successful
// promotion instead (ideally as an execution test next to
// value_promotion_runtime_var_tail.ks).
module Test

enum E { case Bad }

func computedTail(y: std.numeric.Int64) -> std.numeric.Int64 throws E {
    y + 1 // ERROR: expected Result
}
