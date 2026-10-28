// test: diagnostics
// stdlib: true

// A ref-only member (read provider only, no `set`/`mutating ref`) is a
// read-only place: assignment and RMW are rejected. The RMW rejection is
// E207 — the read provider's `&T` is a shared ref, and mutating through
// it is the const-cast hole stage 1 closed.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Buf {
    var p: Pointer[Int64]
    subscript(at index: Int64) -> Int64 {
        ref { self.p.offset(by: index).value }
    }
}

func use(b: Buf) {
    var b2 = b;
    b2(at: 0) = 9; // ERROR(E201)
    b2(at: 0) += 1; // ERROR(E207)
}
