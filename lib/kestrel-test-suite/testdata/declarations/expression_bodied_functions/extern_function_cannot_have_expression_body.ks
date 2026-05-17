// test: diagnostics
// stdlib: true

module Test

struct MyInt: Prelude.FFISafe { }

@extern(.C)
func external() -> MyInt = MyInt() // ERROR: cannot have a body
