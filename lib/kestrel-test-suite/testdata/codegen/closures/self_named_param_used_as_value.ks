// test: execution
// stdlib: true

// G3, `self` half: `self` is not a lexer/parser keyword, so a free function may
// legally declare a parameter named `self`. The thunk pass used to filter out
// every parameter named `self` at ANY position, so this function lost its first
// parameter and the generated thunk called a 2-param target with 1 argument —
// a backend verifier failure, not a diagnostic. Forwarding is now positional.

module Test

func combine(self: std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    self * 100 + x
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
