// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// Regression (#164): reading a Copyable field through a tuple element of a
// non-Copyable type (`t.0.id`) is a place borrow, not a move-out-of-borrow.
// Before the fix `emit_tuple_extract` always copied the extracted element, so a
// non-Copyable element hit the move-out-of-borrow backstop (false E503) — even
// though the isomorphic struct path (`w.inner.id`) compiled. The fix mirrors
// `emit_struct_extract`'s non-Copyable guard: hand back the @guaranteed view.
// Also covers a borrowing method call through a tuple element and confirms each
// element is dropped exactly once (no double-free).

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { drops = drops + 1; }
    func peek() -> Int64 { self.id }
}

@main
func main() -> lang.i64 {
    if true {
        let t = (Res(id: 1), Res(id: 2));
        let v = t.0.id;        // Copyable field read through a non-Copyable element
        let p = t.1.peek();    // borrowing method call through a tuple element
        if v != 1 { return 1 }
        if p != 2 { return 2 }
    }
    // both elements dropped exactly once at scope exit — no double-free
    if drops != 2 { return 3 }
    return 0;
}
