// test: diagnostics
// stdlib: true

module Test
enum Foo {
    case A
    var x: std.numeric.Int64 // ERROR: enums cannot have stored fields
}
