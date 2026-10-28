// test: diagnostics
// stdlib: true

// #169: out-of-range typed integer literals must be a hard error (E121),
// not a silent truncate/wrap. Covers positive overflow, negative underflow,
// the unsigned-overflow case that used to collide with `Int64.minValue`
// (2^63), and an unsigned type. A leading `-` is the `negate` operator over
// the literal, so `-129` is checked as the negated value.

module Test

import std.numeric.(Int8, Int64, UInt8)

func overflows() {
    let a: Int8 = 200;                   // ERROR: integer literal out of range for `Int8`
    let b: Int8 = -129;                  // ERROR: integer literal out of range for `Int8`
    let c: Int64 = 9223372036854775808;  // ERROR: integer literal out of range for `Int64`
    let g: UInt8 = 300;                  // ERROR: integer literal out of range for `UInt8`
}

// Boundary values are in range and must NOT be flagged.
func boundaries() {
    let a: Int8 = -128;
    let b: Int8 = 127;
    let g: UInt8 = 255;
    let c: Int64 = 9223372036854775807;
}
