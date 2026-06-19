// test: execution
// stdlib: true
// backends: cranelift,llvm

// Literal-element decay (stage 1.5, item-5 follow-up): a ref-returning
// call as an array/tuple/dict literal ELEMENT decays to an owned copy —
// the aggregate owns its elements. Mixed and all-ref element lists both
// produce owned aggregates; mutating the source afterwards proves no
// aliasing. Cloneable elements clone exactly once per ref element.
module Test

import std.collections.(Dictionary)
import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Box {
    var v: Int64
    var w: Int64
    func peekV() -> &Int64 { self.v }
    func peekW() -> &Int64 { self.w }
}

struct Tracked: Cloneable {
    var payload: String
    var clones: Pointer[Int64]
    func clone() -> Tracked {
        self.clones.write(self.clones.read() + 1);
        Tracked(payload: self.payload.clone(), clones: self.clones)
    }
}

struct THolder {
    var t: Tracked
    func peek() -> &Tracked { self.t }
}

@main
func main() -> lang.i64 {
    var b = Box(v: 10, w: 20);

    // Array literal: mixed ref + plain elements.
    let xs = [b.peekV(), 5, b.peekW()];
    if xs(0) != 10 { return 1; }
    if xs(1) != 5 { return 2; }
    if xs(2) != 20 { return 3; }

    // Owned, not aliased.
    b.v = 99;
    if xs(0) != 10 { return 4; }

    // Tuple literal.
    let pair = (b.peekV(), b.peekW());
    if pair.0 != 99 { return 5; }
    if pair.1 != 20 { return 6; }
    b.w = 1;
    if pair.1 != 20 { return 7; }

    // Dict literal: ref values decay.
    let d: Dictionary[Int64, Int64] = [1: b.peekV(), 2: b.peekW()];
    if d(unwrap: 1) != 99 { return 8; }
    if d(unwrap: 2) != 1 { return 9; }

    // Cloneable element: the decay costs exactly ONE clone over the
    // plain-element baseline. (Array literal construction itself clones
    // Cloneable elements once today — pre-existing storage-init cost,
    // measured as the baseline below so this pin survives if that cost
    // is ever optimized away.)
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    clones.write(0);
    let h = THolder(t: Tracked(payload: "alpha", clones: clones));
    let before = clones.read();
    let plain = [Tracked(payload: "beta", clones: clones)];
    let mid = clones.read();
    let viaRef = [h.peek()];
    let after = clones.read();
    if after - mid != (mid - before) + 1 { return 10; }
    if viaRef(at: 0).payload != "alpha" { return 11; }
    if plain(at: 0).payload != "beta" { return 12; }
    0
}
