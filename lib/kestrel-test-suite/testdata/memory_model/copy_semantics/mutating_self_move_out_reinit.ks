// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#141, facet 2 — the Optional.take()/replace() shape): in a mutating
// method of a non-Copyable type, `let old = self; self = new; return old` must
// move `self` OUT (leaving the slot uninitialized), store the new value as a
// no-drop StoreInit, and return the old value. Before the init-tracking
// completion the self param was never enrolled in init-tracking, so `self = new`
// always took the drop arm and dropped the MOVED-OUT slot — a double-free that
// SIGSEGV'd (cranelift) / SIGTRAP'd + failed to compile a function (llvm). Both
// backends. Also exercises stdlib `Optional.take()` / `replace()` on a
// non-Copyable payload, which lower to exactly this pattern.

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { drops = drops + 1; }
    // move self out, install `n`, hand back the old self
    mutating func swapWith(consuming n: Res) -> Res {
        let old = self;
        self = n;
        return old;
    }
}

func consume(consuming r: Res) {}

@main
func main() -> lang.i64 {
    var v = Res(id: 1);
    let taken = v.swapWith(Res(id: 2));   // taken == id 1, v == id 2, nothing dropped yet
    if drops != 0 { return 1 };
    if taken.id != 1 { return 2 };
    consume(taken);                        // drops id 1
    if drops != 1 { return 3 };
    consume(v);                            // drops id 2
    if drops != 2 { return 4 };

    // stdlib Optional.take(): returns the payload, leaves .None behind
    var opt: Res? = .Some(Res(id: 7));
    let t = opt.take();
    if not opt.isNone() { return 5 };
    if let .Some(p) = t { if p.id != 7 { return 6 } } else { return 7 };

    // stdlib Optional.replace(): returns the OLD value
    var o2: Res? = .Some(Res(id: 5));
    let old = o2.replace(Res(id: 9));
    if let .Some(p) = old { if p.id != 5 { return 8 } } else { return 9 };
    0
}
