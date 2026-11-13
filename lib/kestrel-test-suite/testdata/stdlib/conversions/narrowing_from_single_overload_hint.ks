// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int8, Int64)

// `Int8` has only the identity `init(from: Int8)`. The call still resolves
// through `Convertible[From]`, as the multi-overload case does (see
// narrowing_from_rejected.ks), so the error names the narrowing spellings
// instead of a bare "expected Int8 got Int64".
func narrow(n: Int64) {
    let a = Int8(from: n); // ERROR: use 'Int8(exactly:)', 'Int8(truncating:)' or 'Int8(clamping:)'
}
