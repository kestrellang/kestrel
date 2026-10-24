// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int32)

@extern(.C, mangleName: "close")
func sys_close(fd: Int32) -> Int32

// `not Copyable` values are never implicitly copied, so early drops never
// apply: no warning even though the deinit closes an fd.
struct File: not Copyable {
    var fd: Int32
    deinit { sys_close(self.fd); }
}
