// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression: overwriting a tuple element that holds a non-Copyable value
// (`t.0 = new`) must drop the OLD element exactly once. The MIR `StoreAssign`
// expansion only knew how to drop `Named` pointees; a `Pointer[Tuple]` pointee
// (a tuple element, or a tuple-typed struct field) fell through to a plain
// store-over, leaking the old value. The expand pass now drops a tuple pointee
// recursively, mirroring the DestroyAddr tuple arm. Covers both a direct
// non-Copyable element and a tuple-typed element (pointee is itself a tuple).

module Test

import std.num.Int64

var drops: Int64 = 0;

struct Tracked {
    var id: Int64;
    deinit { drops = drops + 1; }
}

@main
func main() -> Int64 {
    // (a) direct non-Copyable element.
    var t = (Tracked(id: 1), 99);
    t.0 = Tracked(id: 2);          // old Tracked(1) dropped here
    if drops != 1 { return 1 };
    if t.0.id != 2 { return 2 };

    // (b) element that is itself a tuple containing a non-Copyable member —
    // the StoreAssign pointee is Pointer[(Tracked, Int64)].
    drops = 0;
    var u = ((Tracked(id: 10), 1), 0);
    u.0 = (Tracked(id: 20), 2);    // old (Tracked(10), 1) dropped
    if drops != 1 { return 3 };
    if (u.0).0.id != 20 { return 4 };
    if (u.0).1 != 2 { return 5 };
    0
}
