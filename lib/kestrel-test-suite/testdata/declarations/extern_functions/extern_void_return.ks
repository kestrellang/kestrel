// test: diagnostics
// stdlib: true

module Test

struct Ptr: FFISafe {}

@extern(.C)
func free(ptr: Ptr)
