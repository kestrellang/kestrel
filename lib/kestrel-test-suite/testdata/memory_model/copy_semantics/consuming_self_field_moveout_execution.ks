// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#145 / #152): moving a non-Copyable field out of a *consuming*
// receiver is the documented idiom (`copy-semantics.md`):
//   consuming func intoInner() -> Res { self.inner }
// It was mis-lowered as `begin_borrow self → struct_extract (@guaranteed) →
// move_value (bit-copy out of the borrow) → destroy_value self`, which (a) hit
// the move-out-of-borrow E503 backstop (a false positive on consuming self) and
// (b) left the WHOLE self to be dropped — double-freeing the field that was just
// moved out (drops=2). Now the owned receiver is DESTRUCTURED (consumed, no
// whole-self drop), the moved field is handed back @owned, and only the sibling
// fields drop. Covers single-field, multi-field sibling drop, and a heap (String)
// payload (which previously printed a corrupted double-freed buffer).

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { drops = drops + 1; }
}

// One field: the canonical #145/#152 shape.
struct Wrap: not Copyable {
    var inner: Res
    consuming func intoInner() -> Res { self.inner }
}

// Two non-Copyable fields: moving one out must drop EXACTLY the sibling when the
// receiver is consumed, then the returned field drops at its own scope exit.
struct Pair: not Copyable {
    var a: Res
    var b: Res
    consuming func takeA() -> Res { self.a }
}

// Heap payload: a double-free here corrupted the printed bytes before the fix.
struct Named: not Copyable {
    var name: String
    consuming func intoName() -> String { self.name }
}

// Generic field: `inner: T` is mono-dependent (copy behavior unknown pre-mono).
// The move-out gate must fire here too — falling through to the borrow+copy path
// bitwise-aliases the field and double-frees once T resolves non-Copyable (#141
// hazard). Mirrors the var-read `is_non_copyable || mono_dependent` guard.
struct Wrapper[T]: not Copyable {
    var inner: T
    consuming func unwrap() -> T { self.inner }
}

func consume(consuming r: Res) {}

@main
func main() -> lang.i64 {
    // Single field: intoInner moves `inner` out, self is consumed (no extra drop).
    let w = Wrap(inner: Res(id: 8));
    let r = w.intoInner();
    if drops != 0 { return 1 };          // nothing dropped yet — field was MOVED, not copied
    if r.id != 8 { return 2 };
    consume(r);                          // drops the one logical Res exactly once
    if drops != 1 { return 3 };

    // Multi-field: takeA consumes the Pair, returns `a`, drops `b` once.
    drops = 0;
    let p = Pair(a: Res(id: 1), b: Res(id: 2));
    let ra = p.takeA();
    if drops != 1 { return 4 };          // sibling `b` dropped exactly once
    if ra.id != 1 { return 5 };
    consume(ra);                         // returned `a` drops → total 2
    if drops != 2 { return 6 };

    // Heap field: must survive intact (no double-free corruption).
    let n = Named(name: "omega_omega_omega");
    let s = n.intoName();
    if s != "omega_omega_omega" { return 7 };

    // Generic (mono-dependent) field instantiated to a non-Copyable type.
    drops = 0;
    let g = Wrapper[Res](inner: Res(id: 4));
    let gr = g.unwrap();
    if drops != 0 { return 8 };          // moved, not copied
    if gr.id != 4 { return 9 };
    consume(gr);
    if drops != 1 { return 10 };         // exactly one drop, no double-free

    0
}
