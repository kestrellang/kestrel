// test: diagnostics
// stdlib: true

module Test
import Prelude

struct MyInt: FFISafe {}
struct Ptr: FFISafe {}

@extern(.C, mangleName: "read")
func readSocket(fd: MyInt, buf: Ptr, count: MyInt) -> MyInt
