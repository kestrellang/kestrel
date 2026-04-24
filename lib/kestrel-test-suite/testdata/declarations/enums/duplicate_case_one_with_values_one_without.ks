// test: diagnostics
// stdlib: false
module Test
enum Status {
    case Active
    case Active(reason: lang.str) // ERROR: duplicate enum case
}
