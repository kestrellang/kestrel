// test: diagnostics
// stdlib: false
module Test

protocol Comparable {
    func compare(other: Self)
}
protocol Less {
    func lessThan(other: Self)
}
extend Comparable: Less {
    func lessThan(other: Self) { }
}
struct MyInt { }
func requiresLess[T](a: T, b: T) where T: Less {
    a.lessThan(b);
}
func test() {
    let a = MyInt();
    let b = MyInt();
    requiresLess(a, b); // ERROR: does not satisfy constraint
}
