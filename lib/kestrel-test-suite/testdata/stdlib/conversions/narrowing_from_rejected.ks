// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64, UInt64, Float32, Float64)

// `init(from:)` is lossless only: a source that doesn't always fit has no
// `from:`, and the error names the spellings that do exist.
func narrow(n: Int64, u: UInt64, f: Float64) {
    let b = Int64(from: u); // ERROR: use 'Int64(exactly:)', 'Int64(truncating:)' or 'Int64(clamping:)'
    let c = Float64(from: n); // ERROR: use 'Float64(rounding:)'
    let d = Float32(from: f); // ERROR: use 'Float32(rounding:)'
    let e = Int64(from: f); // ERROR: not every 'Float64' fits in 'Int64'
}
