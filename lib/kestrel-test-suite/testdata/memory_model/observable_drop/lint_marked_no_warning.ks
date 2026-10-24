// test: execution
// stdlib: true
// backends: cranelift,llvm
// warnings: checked

// Marked: copies keep their scope-end drop, so no warning.
module Test

import std.numeric.(Int32)

@extern(.C, mangleName: "close")
func sys_close(fd: Int32) -> Int32

@observabledrop
struct Handle: Cloneable {
    var fd: Int32
    func clone() -> Handle { Handle(fd: self.fd) }
    deinit { sys_close(self.fd); }
}

@main
func main() -> lang.i64 {
    let h = Handle(fd: -1);
    0
}
