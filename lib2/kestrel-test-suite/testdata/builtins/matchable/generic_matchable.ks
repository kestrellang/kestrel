// test: diagnostics
// stdlib: false

module Test
struct Box[T] where T: Prelude.Matchable {
    var value: T
}
extend Box[T]: Prelude.Matchable where T: Prelude.Matchable {
    func matches(other: Box[T]) -> lang.i1 {
        self.value.matches(other.value)
    }
}
