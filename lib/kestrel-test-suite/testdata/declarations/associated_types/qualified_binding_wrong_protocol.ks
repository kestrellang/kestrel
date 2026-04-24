// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
protocol Other { }
struct Foo: Iterator, Other {
    type Other.Item = lang.i64; // ERROR: does not have associated type 'Item'
}
