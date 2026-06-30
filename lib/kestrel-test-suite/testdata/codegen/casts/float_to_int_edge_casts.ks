// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#207): `Float64 -> Int64` conversion of NaN / +-inf /
// out-of-range values must be deterministic and identical on both backends.
// The LLVM backend used a non-saturating `fptosi`, which is undefined
// behavior for those inputs at -O2 (nondeterministic garbage); it now uses
// the saturating `llvm.fptosi.sat`, matching Cranelift's `fcvt_to_sint_sat`.
// `Float64.toInt64()` returns `.None` for non-representable inputs and
// `.Some(truncated)` otherwise. `// backends: cranelift,llvm` is load-bearing:
// the bug was LLVM-only, so the test must run on LLVM.

module Test

@main
func main() -> lang.i32 {
    // NaN, +-inf, and out-of-range finite values -> None.
    let nan = Float64.nan;
    if nan.toInt64().isSome() { return 1 };

    let inf = Float64.infinity;
    if inf.toInt64().isSome() { return 2 };

    let neginf = inf.negate();
    if neginf.toInt64().isSome() { return 3 };

    let big: Float64 = 10000000000000000000.0;       // > Int64.max
    if big.toInt64().isSome() { return 4 };

    let nbig: Float64 = -10000000000000000000.0;      // < Int64.min
    if nbig.toInt64().isSome() { return 5 };

    // In-range values truncate toward zero.
    let pos: Float64 = 3.7;
    if let .Some(v) = pos.toInt64() {
        if v != 3 { return 6 }
    } else {
        return 7
    };

    let neg: Float64 = -3.7;
    if let .Some(v) = neg.toInt64() {
        if v != -3 { return 8 }
    } else {
        return 9
    };

    let ok: Float64 = 3000000000000000000.0;
    if let .Some(v) = ok.toInt64() {
        if v != 3000000000000000000 { return 10 }
    } else {
        return 11
    };

    return 0;
}
