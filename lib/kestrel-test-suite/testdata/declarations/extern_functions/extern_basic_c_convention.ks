// test: diagnostics
// stdlib: true

module Test
import Prelude

// Empty structs are trivially FFISafe (no fields to check)
struct MyInt: FFISafe {}
struct Ptr: FFISafe {}

@extern(.C)
func malloc(size: MyInt) -> Ptr
