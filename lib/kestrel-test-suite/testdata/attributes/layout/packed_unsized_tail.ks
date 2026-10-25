// test: diagnostics
// stdlib: true
//
// E910: a packed struct has a fixed size, so it cannot end in an unsized
// tail.

module Main

import std.numeric.(Int64)

@layout(.Packed) // ERROR: '@layout(.Packed)' cannot be applied to a type with an unsized tail
struct PackedTail[T] where T: not Sized {
    var len: Int64;
    var tail: T;
}
