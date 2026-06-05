// test: execution
// stdlib: true

// Promotion at a function-argument coercion site: passing a bare `Int64`
// where the parameter is `Int64?` must wrap via `FromValue.from`. Runtime
// check that the callee sees `.Some(42)`, not garbage.
module Test

func unwrapOr(v: std.result.Optional[std.numeric.Int64], default: std.numeric.Int64) -> std.numeric.Int64 {
    match v {
        some x => x,
        null => default
    }
}

@main
func main() -> lang.i64 {
    if unwrapOr(42, 0) != 42 { return 1 }
    if unwrapOr(null, 7) != 7 { return 2 }
    0
}
