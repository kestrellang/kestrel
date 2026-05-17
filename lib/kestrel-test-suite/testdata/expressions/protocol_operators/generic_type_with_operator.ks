// test: diagnostics
// stdlib: false
// include: operator_prelude.ks

module Test
struct Wrapper[T] where T: Prelude.AddOperatorProtocol {
    var inner: T
}
extend Wrapper[T]: Prelude.AddOperatorProtocol where T: Prelude.AddOperatorProtocol {
    func add(rhs: Wrapper[T]) -> Wrapper[T] {
        Wrapper[T](inner: self.inner.add(rhs.inner))
    }
}
