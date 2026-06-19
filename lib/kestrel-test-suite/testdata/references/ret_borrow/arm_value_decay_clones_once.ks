// test: execution
// stdlib: true
// backends: cranelift,llvm

// Arm-value decay (stage 1.5): the decay copy of a Cloneable pointee runs
// exactly ONCE, in the taken arm only — the untaken ref arm clones nothing,
// and a non-ref arm clones nothing.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Tracked: Cloneable {
    var payload: String
    var clones: Pointer[Int64]
    func clone() -> Tracked {
        self.clones.write(self.clones.read() + 1);
        Tracked(payload: self.payload.clone(), clones: self.clones)
    }
}

struct Holder {
    var t: Tracked
    func peek() -> &Tracked { self.t }
}

func pick(h: Holder, clones: Pointer[Int64], c: Int64) -> Tracked {
    match c {
        1 => h.peek(),
        _ => Tracked(payload: "other", clones: clones),
    }
}

func pickRefs(h: Holder, c: Int64) -> Tracked {
    if c == 1 { h.peek() } else { h.peek() }
}

@main
func main() -> lang.i64 {
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    clones.write(0);

    let h = Holder(t: Tracked(payload: "alpha", clones: clones));
    let baseline = clones.read();

    // Taken ref arm: exactly one clone (the decay copy).
    let a = pick(h, clones, 1);
    if clones.read() != baseline + 1 { return 1; }
    if a.payload != "alpha" { return 2; }

    // Untaken ref arm: zero clones (construction is not a clone).
    let b = pick(h, clones, 0);
    if clones.read() != baseline + 1 { return 3; }
    if b.payload != "other" { return 4; }

    // All arms refs: still exactly one clone for the taken arm.
    let c = pickRefs(h, 1);
    if clones.read() != baseline + 2 { return 5; }
    if c.payload != "alpha" { return 6; }
    0
}
