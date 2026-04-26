// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
struct Foo: Iterator {
    type Item; // ERROR: type alias requires a type definition
}
