// test: diagnostics
// stdlib: true

module Test

struct MyInt: FFISafe {}

@extern(.C)
func mutatingParam(mutating x: MyInt) -> MyInt // ERROR: consuming
