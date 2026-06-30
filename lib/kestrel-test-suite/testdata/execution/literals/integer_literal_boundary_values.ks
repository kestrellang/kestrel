// test: execution
// stdlib: true
// expect-exit: 0

// #169: fixed-width integer literals at the exact min/max of their type must
// compile and round-trip. In particular `UInt64.maxValue` (2^64-1) and
// `Int64.minValue` (-2^63) exercise the `i128` literal representation that
// replaced the lossy `i64` bit-cast — without it the max/min would not survive
// lowering.

module Test

import std.numeric.(Int8, UInt8, Int64, UInt64)

@main
func main() -> lang.i32 {
    let i8min: Int8 = -128;
    let i8max: Int8 = 127;
    let u8max: UInt8 = 255;
    let i64min: Int64 = -9223372036854775808;
    let i64max: Int64 = 9223372036854775807;
    let u64max: UInt64 = 18446744073709551615;

    if i8min != -128 { return 1 }
    if i8max != 127 { return 2 }
    if u8max != 255 { return 3 }
    if i64min != Int64.minValue { return 4 }
    if i64max != Int64.maxValue { return 5 }
    if u64max != UInt64.maxValue { return 6 }
    0
}
