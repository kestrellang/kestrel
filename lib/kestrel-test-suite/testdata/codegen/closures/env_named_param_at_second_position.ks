// test: execution
// stdlib: true

// G3, non-leading position: the old name filter dropped `env` wherever it
// appeared, not just at index 0. Here `needs_env` was false but the filter
// still removed the second parameter, so the thunk called a 2-param target
// with 1 argument. Forwarding is now positional and position-independent.

module Test

func combine(x: std.numeric.Int64, env: std.numeric.Int64) -> std.numeric.Int64 {
    x * 100 + env
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
