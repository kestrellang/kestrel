// test: diagnostics
// stdlib: false

module Test
@builtin(.Bool)
@builtin(.Bool) // ERROR: duplicate attribute '@builtin'
struct MyBool {}
