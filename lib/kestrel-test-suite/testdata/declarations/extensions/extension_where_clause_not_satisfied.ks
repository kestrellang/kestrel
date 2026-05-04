// test: diagnostics
// stdlib: false
module Test
protocol Equatable { func isEqual(to other: Self) -> lang.i1 }
struct NotEquatable { }
struct Box[T] { var value: T }
extend Box[T] where T: Equatable {
    func hasSameValue(other: Box[T]) -> lang.i1 { return self.value.isEqual(to: other.value); }
}
func test() -> lang.i1 {
    let b1 = Box[NotEquatable](value: NotEquatable());
    let b2 = Box[NotEquatable](value: NotEquatable());
    return b1.hasSameValue(b2); // ERROR: hasSameValue
}
