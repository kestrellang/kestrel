// test: execution
// stdlib: true
// expect-exit: 0

// G13, the guard on the OTHER side of the fix. Teaching `type_satisfies` to
// enforce `where T: Copyable` (see
// `stdlib/rcbox/get_value_rejects_noncopyable_payload.ks`) must not make it
// reject when the substituted argument is ABSTRACT.
//
// `type_satisfies` is a best-effort completeness gate whose module contract is
// "reject only on a provable CONCRETE violation; permit every abstract
// position". The copy-semantics classifier does not share that contract — it
// can return a definite `NotCopyable` for a `not Copyable`-bounded `Param` or
// a `some P and not Copyable` `Opaque` — so the new arm permits
// Param / SelfType / AssocProjection / Opaque / Infer / Error BEFORE
// delegating. Per-instantiation precision stays the solver's job
// (`type_conforms_copyable`).
//
// Each case below reaches a `where T: Copyable`-gated member with a
// non-concrete argument:
//
//   Wrap.read()  — `self.inner.fetch()` where `inner: Cell[T]` and `T` is the
//                  enclosing extension's own Param. The user-level analog of
//                  lang/std/memory/cowbox.ks:52, whose `read()` calls
//                  `self.inner.getValue()` on `RcBox[T]` — the exact call the
//                  permit keeps alive in the shipped stdlib.
//   CowBox.read() — that stdlib call itself.
//   Cell.one()   — the same bound gating a whole added CONFORMANCE
//                  (`extend Cell[T]: HasOne where T: Copyable`) rather than a
//                  bare member, so the frontend has to permit it on the
//                  conformance-selection path too.
//
// Deliberately NOT covered here: reaching `one()` through a generic
// `where T: HasOne` bound. That routes into the mono witness selector, whose
// `type_conforms_at_mono` (kestrel-mir/src/mono/witness.rs) answers a
// `where T: Copyable` constraint by searching for a WITNESS for `Copyable` —
// which structurally never exists, since Copyable is copy-semantics, not a
// declared conformance. It therefore rejects `Cell[Int64]: HasOne` with
// "no matching conformance for this instantiation". That is a SEPARATE,
// pre-existing gap on a code path this fix does not touch (it is mono's own
// independent answer to "is this Copyable", not `type_satisfies`'), and the
// `where T: Equatable` spelling of the identical program compiles and runs.
// Tracked apart from G13.

module Test

import std.numeric.Int64
import std.text.String
import std.memory.CowBox

struct Cell[T] {
    var v: T;
}

extend Cell[T] where T: Copyable {
    public func fetch() -> T { self.v }
}

struct Wrap[T] {
    var inner: Cell[T];
}

extend Wrap[T] {
    // `T` is an abstract Param here, not a concrete type: the `where T: Copyable`
    // bound on `fetch` is checked in an abstract position and must permit.
    public func read() -> T { self.inner.fetch() }
}

protocol HasOne {
    func one() -> Int64
}

extend Cell[T]: HasOne where T: Copyable {
    public func one() -> Int64 { 1 }
}

@main
func main() -> lang.i32 {
    let w = Wrap[Int64](inner: Cell[Int64](v: 6));
    if w.read() != 6 { return 10 }

    // The shipped stdlib path: `CowBox[T].read()` -> `RcBox[T].getValue()`,
    // gated by `extend RcBox[T] where T: Copyable`, with `T` abstract.
    var cb = CowBox[String]("hi");
    if cb.read() != "hi" { return 11 }

    let c = Cell[Int64](v: 2);
    if c.one() != 1 { return 12 }

    0
}
