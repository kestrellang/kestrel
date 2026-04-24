// test: diagnostics
// stdlib: false
module Test

protocol Equatable { }
protocol Iterator {
    type Item;
}
protocol Container {
    type Iter: Iterator;
}
func findIn[C](c: C) where C: Container, C.Iter.Item: Equatable { }
