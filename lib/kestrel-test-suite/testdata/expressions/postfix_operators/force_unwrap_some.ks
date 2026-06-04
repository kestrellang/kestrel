// test: execution
// stdlib: true

// Regression: postfix `!` force-unwrap must compile and yield the wrapped
// value. Previously this ICE'd at MIR OSSA verify because the desugared
// `.None` arm was `HirExpr::Error` (type `Error`, not `Never`), producing a
// block-arg type mismatch at the match merge. `!` now lowers to a
// `ForceUnwrap.forceUnwrap()` ProtocolCall returning the wrapped value.

module Test

func main() -> lang.i64 {
    let opt: std.result.Optional[std.numeric.Int64] = .Some(42);

    // direct unwrap
    let v = opt!;
    if v != 42 { return 1 }

    // unwrap inside a larger expression
    let sum = opt! + 8;
    if sum != 50 { return 2 }

    // unwrap a freshly-constructed optional
    let nested: std.result.Optional[std.numeric.Int64] = .Some(7);
    let n = nested!;
    if n != 7 { return 3 }

    0
}
