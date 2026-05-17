// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
struct ArrayIterator[T]: Iterator {
    type Item = T;
}
