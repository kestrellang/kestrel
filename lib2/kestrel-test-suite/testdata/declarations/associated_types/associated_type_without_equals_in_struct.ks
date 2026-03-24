// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
struct Foo: Iterator {
    type Item; // ERROR: associated type binding requires a type
}
