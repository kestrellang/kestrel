// test: execution
// stdlib: true

// G3, `_env` half: the thunk pass matched BOTH `env` and `_env` by name, so a
// user parameter spelled `_env` miscompiled exactly like `env` (returned 3
// instead of 307). `_env` is the name the thunk pass gives its OWN synthesized
// env parameter; that is a fact about generated code, not about user code.

module Test

func combine(_env: std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    _env * 100 + x
}

func apply(
    f: (std.numeric.Int64, std.numeric.Int64) -> std.numeric.Int64,
    a: std.numeric.Int64,
    b: std.numeric.Int64,
) -> std.numeric.Int64 {
    f(a, b)
}

@main
func main() -> lang.i64 {
    if apply(combine, 3, 7) != 307 { return 1 }
    0
}
