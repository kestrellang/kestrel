// test: execution
// stdlib: true

// G3: a user parameter named `env` is a normal parameter. The thunk pass used
// to decide "params[0] is the closure environment pointer" by matching the
// parameter's NAME, so passing `combine` as a function value forwarded the env
// pointer into `env: Int64` and dropped `x` entirely — a silent miscompile that
// returned 3 instead of 307. Whether params[0] is an env pointer is now read
// off `FunctionKind`, which is what the producers actually set.

module Test

func combine(env: std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    env * 100 + x
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
