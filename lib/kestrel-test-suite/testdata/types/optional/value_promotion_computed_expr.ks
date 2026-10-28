// test: execution
// stdlib: true
// expect-exit: 0

// Value promotion (`T` -> `Result[T,E]` / `Optional[T]`) fires for a
// COMPUTED-EXPRESSION tail/return (`y + 1`, a call result), not only for bare
// values. Formerly the "3-i" known bug: `solve_coerce` unified the still-
// unresolved operator result with the target before the operator's own
// constraint typed it, so promotion never applied. Check-mode leaves now wait
// for their synthesized type (bidi P3, `InferCtx::synth_pending_exprs`).
module Test

enum E { case Bad }

func computedTail(y: std.numeric.Int64) -> std.numeric.Int64 throws E {
    y + 1
}

func computedOptional(y: std.numeric.Int64) -> std.numeric.Int64? {
    y * 3
}

@main
func main() -> lang.i64 {
    match computedTail(41) {
        .Ok(v) => if v != 42 { return 1; },
        .Err(_) => return 2,
    }
    if computedOptional(2) != .Some(6) { return 3; }
    0
}
