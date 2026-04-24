// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
protocol Container {
    type Iter: Iterator;
}
func intContainer[C](c: C) where C: Container, C.Iter.Item = lang.i64 { }
