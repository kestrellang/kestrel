// test: diagnostics
// stdlib: true

module Test

import std.ffi.FFISafe

struct MyInt: FFISafe { }

@extern(.C)
func external() -> MyInt = MyInt() // ERROR: cannot have a body
