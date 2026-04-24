// test: diagnostics
// stdlib: false

module Test

enum Point {
    case Location(x: lang.i64, x: lang.i64) // ERROR: duplicate label
}
