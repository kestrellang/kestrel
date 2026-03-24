// test: diagnostics
// stdlib: false
module Test

protocol Equatable {
    func eq(other: Self) -> lang.i1
}
protocol Iterator {
    type Item;
    func next() -> Item
}
struct MyInt: Equatable {
    func eq(other: MyInt) -> lang.i1 { true }
}
struct IntIterator: Iterator {
    type Item = MyInt;
    func next() -> MyInt { MyInt() }
}
func contains[T](iter: T, value: T.Item) -> lang.i1 where T: Iterator, T.Item: Equatable {
    true
}
