// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#139): `o.proxy.field = v` where `proxy` is a value-returning
// get/set computed property used to call the getter into a temp, mutate the
// temp's field, then drop it — the setter was never called, silently losing
// the write. It is now a get→modify→set rewrite: the getter copies `proxy`
// into a slot, the field store mutates the slot, and the writeback drain
// calls the setter. Verifies the setter runs exactly once, sibling fields
// survive the round-trip, and compound assignment through the proxy works.

module Test

import std.num.Int64

struct Inner {
    var x: Int64;
    var y: Int64;
}

struct Outer {
    var stored: Inner;
    var setCount: Int64;

    var proxy: Inner {
        get { self.stored }
        set {
            self.stored = newValue;
            self.setCount = self.setCount + 1;
        }
    }
}

@main
func main() -> Int64 {
    var o = Outer(stored: Inner(x: 1, y: 2), setCount: 0);

    o.proxy.x = 42;
    if o.stored.x != 42 { return 1 };
    if o.stored.y != 2 { return 2 };   // sibling field preserved through get/set
    if o.setCount != 1 { return 3 };   // setter called exactly once

    o.proxy.y = o.proxy.y + 100;       // compound: get on RHS, get/modify/set on LHS
    if o.stored.y != 102 { return 4 };
    if o.stored.x != 42 { return 5 };
    if o.setCount != 2 { return 6 };
    0
}
