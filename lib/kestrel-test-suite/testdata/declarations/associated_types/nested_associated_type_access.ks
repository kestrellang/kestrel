// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
protocol Container {
    type Iter: Iterator;
}
func getItem[C](c: C, item: C.Iter.Item) -> C.Iter.Item where C: Container { item }
