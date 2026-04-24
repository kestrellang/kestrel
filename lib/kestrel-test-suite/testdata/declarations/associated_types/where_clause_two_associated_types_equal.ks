// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
func zip[A, B](a: A, b: B) where A: Iterator, B: Iterator, A.Item = B.Item { }
