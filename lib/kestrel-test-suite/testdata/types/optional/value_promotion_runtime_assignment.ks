// test: execution
// stdlib: true

// Promotion at an assignment-coercion site: `x = 42` where `x: Int64?`
// must wrap via `FromValue.from`. Also covers var-init separately from
// the let-binding case.
module Test

func main() -> lang.i64 {
    var v: std.result.Optional[std.numeric.Int64] = null;
    v = 99;
    match v {
        some 99 => 0,
        _ => 1
    }
}
