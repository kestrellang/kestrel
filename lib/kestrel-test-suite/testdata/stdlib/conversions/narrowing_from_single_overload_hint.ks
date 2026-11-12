// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int8, Int64)

// `Int8` has only the identity `init(from: Int8)`, so this call resolves to
// that one init directly and fails as a plain argument mismatch. It should
// name the narrowing spellings, as the multi-overload case does (see
// narrowing_from_rejected.ks). Fails today: the error is a bare E109.
func narrow(n: Int64) {
    let a = Int8(from: n); // ERROR: use 'Int8(exactly:)', 'Int8(truncating:)' or 'Int8(clamping:)'
}
