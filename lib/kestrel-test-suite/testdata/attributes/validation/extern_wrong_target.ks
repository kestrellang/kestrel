// test: diagnostics
// stdlib: false

module Test
struct S {
    @extern(.C) // ERROR: '@extern' cannot be applied to a method; it applies to functions
    func m()
}
