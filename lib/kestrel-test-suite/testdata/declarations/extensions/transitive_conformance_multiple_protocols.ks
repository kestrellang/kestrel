// test: diagnostics
// stdlib: false
module Test

protocol Comparable {
    func compare(other: Self)
}
protocol Less {
    func lessThan(other: Self)
}
protocol Greater {
    func greaterThan(other: Self)
}
extend Comparable: Less, Greater {
    func lessThan(other: Self) { }
    func greaterThan(other: Self) { }
}
struct MyInt: Comparable {
    func compare(other: MyInt) { }
}
func requiresLess[T](a: T, b: T) where T: Less {
    a.lessThan(b);
}
func requiresGreater[T](a: T, b: T) where T: Greater {
    a.greaterThan(b);
}
func test() {
    let a = MyInt();
    let b = MyInt();
    requiresLess(a, b);
    requiresGreater(a, b);
}
