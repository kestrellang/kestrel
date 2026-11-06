// test: execution
// stdlib: true
// backends: cranelift,llvm

// A Cloneable handle whose deinit closes an fd is observable when dropped:
// copy lifetime passes could drop a copy early. E508 (emitted after
// monomorphization, so this is an execution test) asks for the attribute.
module Test

import std.numeric.(Int32)

@extern(.C, mangleName: "close")
func sys_close(fd: Int32) -> Int32

struct Handle: Cloneable { // WARN: `Cloneable` type `Handle` has a `deinit` that calls `sys_close`; mark it `@observabledrop`
    var fd: Int32
    func clone() -> Handle { Handle(fd: self.fd) }
    deinit { sys_close(self.fd); }
}

@main
func main() -> lang.i64 {
    let h = Handle(fd: -1);   // closing -1 is a harmless EBADF
    0
}
