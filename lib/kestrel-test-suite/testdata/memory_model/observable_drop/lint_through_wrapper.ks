// test: execution
// stdlib: true
// backends: cranelift,llvm

// The extern is reached through a Kestrel wrapper; the warning names the
// first named function the deinit calls on the way.
module Test

import std.numeric.(Int32)

@extern(.C, mangleName: "close")
func sys_close(fd: Int32) -> Int32

func closeFd(fd: Int32) -> Int32 {
    sys_close(fd)
}

struct Handle: Cloneable { // WARN: has a `deinit` that calls `closeFd`
    var fd: Int32
    func clone() -> Handle { Handle(fd: self.fd) }
    deinit { closeFd(self.fd); }
}

@main
func main() -> lang.i64 {
    let h = Handle(fd: -1);
    0
}
