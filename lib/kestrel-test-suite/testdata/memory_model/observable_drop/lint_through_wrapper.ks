// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int32)

@extern(.C, mangleName: "close")
func sys_close(fd: Int32) -> Int32

func closeFd(fd: Int32) -> Int32 {
    sys_close(fd)
}

// The extern is reached through a Kestrel wrapper; the warning names the
// function the deinit calls directly.
struct Handle: Cloneable { // WARN: has a `deinit` that calls `closeFd`
    var fd: Int32
    func clone() -> Handle { Handle(fd: self.fd) }
    deinit { closeFd(self.fd); }
}
