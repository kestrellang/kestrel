// test: diagnostics
// stdlib: false

module Test

struct Foo {
    var x: lang.i64
    var x: lang.str // ERROR: duplicate
}
