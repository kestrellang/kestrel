// test: diagnostics
// stdlib: true
//
// E403: a `not Sized` type parameter can only be used behind a pointer or as
// the last stored field of a struct (design §2). A borrowed parameter counts
// as behind a pointer: it is passed by address.

module Test

import std.numeric.(Int64)
import std.memory.(Pointer)

struct Holder[T] where T: not Sized {
    var head: T; // ERROR: 'T' may be unsized here ('where T: not Sized'), so it can only be used behind a pointer or as the last stored field
    var p: Pointer[T];
}

// Legal: T is the unsized tail.
struct Tail[T] where T: not Sized {
    var count: Int64;
    var value: T;
}

// Legal: a borrowed param is passed by address, so it is behind a pointer.
func byBorrow[T](x: T) where T: not Sized {}

func byParam[T](consuming x: T) where T: not Sized {} // ERROR: 'T' may be unsized here ('where T: not Sized'), so it can only be used behind a pointer or as the last stored field

func byMutating[T](mutating x: T) where T: not Sized {} // ERROR: 'T' may be unsized here ('where T: not Sized'), so it can only be used behind a pointer or as the last stored field

func byReturn[T](p: Pointer[T]) -> T where T: not Sized { fatalError("unreachable") } // ERROR: 'T' may be unsized here ('where T: not Sized'), so it can only be used behind a pointer or as the last stored field
