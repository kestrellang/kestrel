// test: diagnostics
// stdlib: true

module Test

import std.memory.(Buffer, Pointer, SystemAllocator)

// Unchecked element access can read or write any address, so it's unsafe:
// `Slice.lend(start: nullPointer, count: 10)(3)` and `xs(unchecked: 10^8)`
// once compiled with no `unsafe` and segfaulted.
func uses(xs: Array[Int64], buffer: Buffer[Int64, SystemAllocator]) {
    let a = xs(unchecked: 0); // ERROR: 'Array.subscript' is unsafe and requires an `unsafe` expression or block
    let b = unsafe xs(unchecked: 0);
    let s = Slice.lend(start: Pointer[Int64].nullPointer(), count: 10); // ERROR: 'lend' is unsafe and requires an `unsafe` expression or block
    let c = buffer.read(at: 0); // ERROR: 'read' is unsafe and requires an `unsafe` expression or block
    let d = buffer.write(at: 0, 5); // ERROR: 'write' is unsafe and requires an `unsafe` expression or block
}
