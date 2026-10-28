// test: execution
// stdlib: true
// backends: cranelift,llvm
// warnings: checked

// `not Copyable` values are never implicitly copied, so early drops never
// apply: no warning even though the deinit closes an fd.
module Test

import std.numeric.(Int32)

@extern(.C, mangleName: "close")
func sys_close(fd: Int32) -> Int32

struct File: not Copyable {
    var fd: Int32
    deinit { sys_close(self.fd); }
}

@main
func main() -> lang.i64 {
    let f = File(fd: -1);
    0
}
