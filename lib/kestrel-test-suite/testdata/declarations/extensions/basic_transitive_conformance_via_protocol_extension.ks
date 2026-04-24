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
struct MyInt: Comparable {
    func compare(other: MyInt) { }
}
// This function requires T: Less, and MyInt should satisfy it
// because MyInt: Comparable and extend Comparable: Less
func requiresLess[T](a: T, b: T) where T: Less {
    a.lessThan(b);
}
func test() {
    let a = MyInt();
    let b = MyInt();
    requiresLess(a, b);
}
