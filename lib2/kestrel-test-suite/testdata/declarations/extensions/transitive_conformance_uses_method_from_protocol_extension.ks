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
func test() {
    let a = MyInt();
    let b = MyInt();
    // Direct call to the method provided by protocol extension
    a.lessThan(b);
}
