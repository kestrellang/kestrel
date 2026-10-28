// test: diagnostics
// stdlib: false

module Test

// `extend Box[T]` is still the generic type: no stored statics.

struct Box[T] { var v: T }

extend Box[T] {
    static var count: lang.i64 = 0; // ERROR: static stored properties not supported in generic types
}
