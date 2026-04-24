// test: diagnostics
// stdlib: false
module Test
enum Color {
    case Red
    case Red // ERROR: duplicate enum case
}
