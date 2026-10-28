// test: diagnostics
//
// An explicitly-typed receiver whose element contradicts the bound's
// protocol args must be rejected in the frontend — under concrete
// per-element conformances no declared instantiation can satisfy the
// bound, so this must not survive to a mono witness-lookup failure
// (or, with a free-param conformance wildcard, a wrong-layout read).

module Test

import std.core.ClosedRange
import std.numeric.(Int8, Int64)

protocol Bounds[B] {
    func probeBounds() -> ClosedRange[B]
}

extend ClosedRange[Int64]: Bounds[Int64] {
    public func probeBounds() -> ClosedRange[Int64] { self }
}

extend ClosedRange[Int8]: Bounds[Int8] {
    public func probeBounds() -> ClosedRange[Int8] { self }
}

func spread8[R](range: R) -> Int8 where R: Bounds[Int8] {
    let bounds = range.probeBounds();
    bounds.end - bounds.start
}

@main
func main() {
    let r: ClosedRange[Int64] = 3..=9;
    let x = spread8(r); // ERROR: conform
    let _ = x;
}
