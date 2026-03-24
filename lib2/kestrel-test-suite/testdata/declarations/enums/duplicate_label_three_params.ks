// test: diagnostics
// stdlib: false

module Test

enum Bad {
    case Triple(a: lang.i64, b: lang.str, a: lang.i1) // ERROR: duplicate label
}
