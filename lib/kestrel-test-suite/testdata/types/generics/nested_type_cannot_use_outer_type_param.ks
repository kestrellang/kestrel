// test: diagnostics
// stdlib: true

// Fuzz M13: nested types do not inherit their container's type parameters.
// Using the outer `T` inside a nested struct/enum (fields, payloads, or the
// nested type's method bodies) is E634.
module Test

import std.numeric.(Int64)

struct Outer[T] {
    struct Inner[U] { var t: T; var u: U } // ERROR(E634)
    enum E {
        case A(T) // ERROR(E634)
    }
    var t: T
}

struct Plain[T] {
    struct M {
        func f() -> Int64 {
            let z: T = 1; // ERROR(E634)
            0
        }
    }
}
