// test: diagnostics

// A witness's parameter types must be the requirement's. Only arity and
// labels used to be compared, so `f(x: String)` satisfied `f(x: Int64)` and
// a generic caller passed an `Int64` where a `String` was expected.

module Test

protocol P { func f(x: Int64) -> Int64 }

struct S: P {
    func f(x: String) -> Int64 { 0 } // ERROR(E632)
}
