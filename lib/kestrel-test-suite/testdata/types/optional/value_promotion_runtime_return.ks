// test: execution
// stdlib: true

// Promotion at a return-coercion site: `return 42` in a `-> Int64?` body
// must wrap via `FromValue.from`. Tests both the tail-expression path and
// the explicit `return` path.
module Test

func tailReturn() -> std.result.Optional[std.numeric.Int64] {
    42
}

func explicitReturn(early: std.core.Bool) -> std.result.Optional[std.numeric.Int64] {
    if early {
        return 7
    }
    42
}

@main
func main() -> lang.i64 {
    match tailReturn() {
        some 42 => {},
        _ => return 1
    }
    match explicitReturn(true) {
        some 7 => {},
        _ => return 2
    }
    match explicitReturn(false) {
        some 42 => {},
        _ => return 3
    }
    0
}
