// test: diagnostics
// stdlib: true

module Test
enum Foo {
    case A
    let x: std.num.Int64 // ERROR: enums cannot have stored fields
}
