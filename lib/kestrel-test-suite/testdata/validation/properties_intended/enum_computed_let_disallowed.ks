// test: diagnostics
// stdlib: true

module Test
enum Foo {
    case A
    let computed: std.numeric.Int64 { 0 } // ERROR: computed properties must use 'var'
}
