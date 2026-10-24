// test: diagnostics
// stdlib: true

module Test

import std.ffi.(RawPointer, free)

// Releasing memory is not observable: no warning.
struct Block: Cloneable {
    var ptr: RawPointer
    func clone() -> Block { Block(ptr: self.ptr) }
    deinit { free(self.ptr); }
}
