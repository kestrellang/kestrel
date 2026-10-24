// test: diagnostics
// stdlib: true
//
// E402: `B` in `any[B] P` must have one `not Sized` type parameter, conform
// to `ExistentialBox`, and hold `T` only behind a pointer (design §3).

module Test

import std.numeric.(Int64)
import std.core.(Indirection)
import std.memory.(ExistentialBox, Pointer)

protocol Shape {
    func area() -> Int64
}

struct Circle { var r: Int64; }
extend Circle: Shape {
    public func area() -> Int64 { 3 * self.r * self.r }
}

struct Square { var side: Int64; }
extend Square: Shape {
    public func area() -> Int64 { self.side * self.side }
}

// Not an ExistentialBox.
struct NotABox[T] where T: not Sized {
    var p: Pointer[T];
}

// Stores T by value (as its unsized tail), so it can't be widened.
struct ByValueBox[T] where T: not Sized {
    var items: T;
}
extend ByValueBox[T]: Indirection {
    type Target = T
    public func pointeeRef() -> &T { self.items }
}
extend ByValueBox[T]: ExistentialBox {
    public init(consuming value: T) { self.items = value; }
}

func a(x: any[Array] Shape) {} // ERROR: 'Array' cannot box an existential: its type parameter is not declared 'not Sized'
func b(x: any[NotABox] Shape) {} // ERROR: 'NotABox' cannot box an existential: it does not conform to 'ExistentialBox'
func c(x: any[ByValueBox] Shape) {} // ERROR: 'ByValueBox' cannot box an existential: field 'items' stores 'T' by value
