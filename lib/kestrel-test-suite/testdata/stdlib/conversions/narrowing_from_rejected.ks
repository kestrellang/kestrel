// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int8, Int64, UInt64, Float32, Float64)

// `init(from:)` is lossless only: a source that doesn't always fit has no
// `from:`, so the call site must say what an out-of-range value becomes.
func narrow(n: Int64, u: UInt64, f: Float64) {
    let a = Int8(from: n); // ERROR: type mismatch
    let b = Int64(from: u); // ERROR: type mismatch
    let c = Float64(from: n); // ERROR: type mismatch
    let d = Float32(from: f); // ERROR: type mismatch
    let e = Int64(from: f); // ERROR: type mismatch
}
