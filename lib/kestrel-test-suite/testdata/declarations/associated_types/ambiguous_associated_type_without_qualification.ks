// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
protocol Container {
    type Item;
}
struct Foo: Iterator, Container {
    type Item = lang.i64; // ERROR: ambiguous associated type
}
