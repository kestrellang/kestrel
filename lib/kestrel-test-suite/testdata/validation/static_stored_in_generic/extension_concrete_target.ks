// test: diagnostics
// stdlib: false

module Test

// Swift's rule is about the TYPE, not the extension: `extend Box[Int64]`
// adds to a generic type, so it may not add a stored static either.

struct Box[T] { var v: T }

extend Box[lang.i64] {
    static var count: lang.i64 = 0; // ERROR: static stored properties not supported in generic types
}
