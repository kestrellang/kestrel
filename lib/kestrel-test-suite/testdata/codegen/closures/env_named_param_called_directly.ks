// test: execution
// stdlib: true

// G3 control: an `env`-named first parameter that is only ever called directly
// never reaches the thunk pass (no `ApplyPartial`), so it was always correct.
// It pins the boundary: the bug lived in thunk synthesis, not in signature
// lowering or the direct-call path.

module Test

func combine(env: std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    env * 100 + x
}

@main
func main() -> lang.i64 {
    if combine(3, 7) != 307 { return 1 }
    0
}
