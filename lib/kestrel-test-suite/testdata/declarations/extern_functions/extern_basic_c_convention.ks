// test: diagnostics
// stdlib: true

module Test

// Empty structs are trivially FFISafe (no fields to check)
struct MyInt: FFISafe {}
struct Ptr: FFISafe {}

@extern(.C)
func malloc(size: MyInt) -> Ptr
