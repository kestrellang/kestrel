// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
struct Foo {
    type Iterator.Item = lang.i64; // ERROR: does not conform to 'Iterator'
}
