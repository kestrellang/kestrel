// test: execution
//
// KNOWN FAILING (2026-07-23) — documents a wanted behavior per the
// project testing policy.
//
// A literal range at a parameterized bound should have its element type
// pinned by the bound's protocol args, not defaulted to Int64: with one
// concrete conformance per element type, `R: Bounds[Int8]` can only be
// satisfied by ClosedRange[Int8], so `3..=9` should infer element Int8.
//
// Why it still fails: `3..=9` gets its type via member dispatch of the
// `..=` operator ON the literal receiver, and member resolution can't run
// until the literal has a concrete type — so literal defaulting (Int64)
// is forced before the ClosedRange type (and its Conforms constraint)
// exists; there is nothing left for the bound to pin. The same pinning
// works when the element flows through an ordinary generic init instead
// (parameterized_bound_literal_pinning_init.ks). Fixing this shape needs
// a builtin fast-path for range operators on integer literals that
// produces `ClosedRange[?e]` structurally, without receiver dispatch.
//
// Today's actual behavior is the CLEAN half of the fix: the call is
// rejected in the frontend ("ClosedRange[Int64] !: Bounds") instead of
// surviving to a mono witness-lookup failure or a wrong-layout read.

module Test

import std.core.ClosedRange
import std.numeric.(Int8, Int32, Int64)

protocol Bounds[B] {
    func probeBounds() -> ClosedRange[B]
}

extend ClosedRange[Int64]: Bounds[Int64] {
    public func probeBounds() -> ClosedRange[Int64] { self }
}

extend ClosedRange[Int32]: Bounds[Int32] {
    public func probeBounds() -> ClosedRange[Int32] { self }
}

extend ClosedRange[Int8]: Bounds[Int8] {
    public func probeBounds() -> ClosedRange[Int8] { self }
}

func spread8[R](range: R) -> Int8 where R: Bounds[Int8] {
    let bounds = range.probeBounds();
    bounds.end - bounds.start
}

@main
func main() -> lang.i64 {
    if spread8(3..=9) != 6 { return 1 }
    0
}
