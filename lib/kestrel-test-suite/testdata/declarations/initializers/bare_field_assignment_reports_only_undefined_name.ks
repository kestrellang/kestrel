// test: diagnostics
// stdlib: true

// Kestrel has no implicit `self`: `x = v` in an initializer names no binding
// (E132). That is the only diagnostic — the target failed to lower, so
// "cannot assign to this expression" (E202) and "initializer does not
// initialize all fields" (E005) would be cascades of the same mistake.

module Test

struct P {
    var x: Int64;

    init(v: Int64) { x = v } // ERROR: undefined name 'x'
}
