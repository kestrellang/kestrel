// test: diagnostics
// stdlib: false

module Test

// Swift's rule: a stored static has one storage cell, so it cannot live in a
// generic type (`Box[Int64].count` and `Box[String].count` would have to be
// either one cell or many). Computed statics are fine.

struct Box[T] {
    static var count: lang.i64 = 0; // ERROR: static stored properties not supported in generic types
    static var computed: lang.i64 { 1 }
}
