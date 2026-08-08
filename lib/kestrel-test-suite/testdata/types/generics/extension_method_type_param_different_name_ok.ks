// test: diagnostics
// stdlib: false
module Test

// Negative guard for the E439 extension case: a method type parameter inside a
// generic extension is only a shadow when it collides with a name the LHS
// introduces. `U` does not, so this must stay clean.

struct Box[T] { var value: T }

extend Box[T] {
    func mapTo[U](f: (T) -> U) -> U {
        f(self.value)
    }
}
