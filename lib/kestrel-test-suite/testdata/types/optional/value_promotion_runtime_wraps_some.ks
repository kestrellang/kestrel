// test: execution
// stdlib: true

// Regression: `let v: Int64? = 42` must wrap 42 into `.Some(42)` via
// `FromValue.from`. Before the fix, MIR-lower ignored the recorded
// promotion entirely — `%v` was left uninitialized and the subsequent
// match read a garbage Optional discriminant, falling into `.None`.
module Test

func main() -> lang.i64 {
    let v: std.result.Optional[std.numeric.Int64] = 42;
    match v {
        some _ => 0,
        null => 1
    }
}
