// test: diagnostics
// stdlib: false

module Test

enum Bad {
    case Foo(x: lang.i64, x: lang.str) // ERROR: duplicate label
}
