// test: diagnostics
// stdlib: true

module Test
import Prelude

struct Ptr: FFISafe {}

@extern(.C)
func free(ptr: Ptr)
