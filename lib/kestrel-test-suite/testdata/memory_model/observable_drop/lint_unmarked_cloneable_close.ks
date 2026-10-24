// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int32)

@extern(.C, mangleName: "close")
func sys_close(fd: Int32) -> Int32

// A Cloneable handle whose deinit closes an fd is observable when dropped:
// copy lifetime passes could drop a copy early. E507 asks for the attribute.
struct Handle: Cloneable { // WARN: `Cloneable` type `Handle` has a `deinit` that calls `sys_close`; mark it `@observabledrop`
    var fd: Int32
    func clone() -> Handle { Handle(fd: self.fd) }
    deinit { sys_close(self.fd); }
}
