// test: execution
// stdlib: true
// backends: cranelift,llvm
// warnings: checked

// Releasing memory is not observable: no warning.
module Test

import std.memory.(RawPointer)
import std.ffi.(malloc, free)

struct Block: Cloneable {
    var ptr: RawPointer
    func clone() -> Block { Block(ptr: unsafe malloc(8)) }
    deinit { unsafe free(self.ptr); }
}

@main
func main() -> lang.i64 {
    let b = Block(ptr: unsafe malloc(8));
    0
}
