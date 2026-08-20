// test: execution
// stdlib: true

// G3 control: identical to `env_named_param_used_as_value.ks` except the first
// parameter is named `e`. This already worked before the fix — it is here so a
// regression in the ORDINARY function-value path is distinguishable from a
// regression in the name-vs-kind decision the other G3 tests cover.

module Test

func combine(e: std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    e * 100 + x
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
