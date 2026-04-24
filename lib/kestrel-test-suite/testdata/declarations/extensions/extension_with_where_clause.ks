// test: diagnostics
// stdlib: false
module Test
protocol Equatable { func equals(other: Self) -> lang.i1 }
struct Box[T] { var value: T }
extend Box[T] where T: Equatable {
    func hasSameValue(other: Box[T]) -> lang.i1 { return self.value.equals(other.value); }
}
