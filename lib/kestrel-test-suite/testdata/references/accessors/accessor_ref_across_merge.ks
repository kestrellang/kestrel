// test: diagnostics
// stdlib: true

// An accessor-fabricated ref obeys the stage-1 merge rule: holding the
// ref open across a control-flow split (a sibling argument with an
// if-expression) is E497, exactly like a ref-returning method call.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Buf {
    var p: Pointer[Int64]
    subscript(at index: Int64) -> Int64 {
        ref { self.p.offset(by: index).value }
    }
}

func add(a: Int64, b: Int64) -> Int64 { a + b }

func use(b: Buf, c: Bool) -> Int64 {
    add(b(at: 0), if c { 1 } else { 2 }) // ERROR(E497)
}
