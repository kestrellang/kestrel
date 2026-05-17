// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
func intOnly[T](iter: T) where T: Iterator, T.Item = lang.i64 { }
