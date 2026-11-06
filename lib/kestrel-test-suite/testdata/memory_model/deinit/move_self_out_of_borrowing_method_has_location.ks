// test: diagnostics

// Returning a borrowed non-Copyable `self` from a method body's tail is
// rejected with E503 at the `self` expression. The error used to have no
// source location.

module Test

struct H: not Copyable {
    var v: Int64
    func me() -> H { self } // ERROR(E503): cannot move non-copyable value of type `Test.H` out of a borrow
}
