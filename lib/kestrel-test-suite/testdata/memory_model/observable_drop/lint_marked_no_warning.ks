// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int32)

@extern(.C, mangleName: "close")
func sys_close(fd: Int32) -> Int32

// Marked: copies keep their scope-end drop, so no warning.
@observabledrop
struct Handle: Cloneable {
    var fd: Int32
    func clone() -> Handle { Handle(fd: self.fd) }
    deinit { sys_close(self.fd); }
}
