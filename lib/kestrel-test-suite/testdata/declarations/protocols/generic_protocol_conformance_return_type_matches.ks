// test: diagnostics
// stdlib: false

// Regression: the conformance checker (E458) failed to unify the protocol's
// type parameter with the conforming type's type parameter, so
// `Wrapper[Proto_T]` != `Wrapper[Impl_T]` even though both represent the
// same `T` positionally.

module Test

struct Wrapper[T] { }

protocol Container[T] {
    func wrap() -> Wrapper[T]
}

struct Box[T] { }

extend Box[T]: Container[T] {
    func wrap() -> Wrapper[T] { Wrapper() }
}

// Also verify the implicit-positional case (no type args on the protocol).
struct Bag[T] { }

extend Bag[T]: Container {
    func wrap() -> Wrapper[T] { Wrapper() }
}
