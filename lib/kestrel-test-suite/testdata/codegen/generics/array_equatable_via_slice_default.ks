// test: execution
// stdlib: true

// Regression: `Array == Array` monomorphization.
//
// `Array[T]: Equatable` is satisfied by the `isEqual` default on
// `extend Slice[T] where T: Equatable` — a protocol-extension default reached
// through a *different* protocol (Equatable) than the one it is defined on
// (Slice). That default's own type param is the Slice element `T`, which
// Equatable's method type args never carry, so the monomorphizer used to
// instantiate `Slice.isEqual` with zero type args and ICE:
//   "type arg arity mismatch for std.collections.Slice.isEqual: expected 1, got 0"
//
// The element type is now recovered from the implementing type's `Slice`
// conformance (`Array[T]: Slice[T]`), so this compiles and runs. Covers Int,
// String (heap element), and nested `Array[Array[T]]` (recursive instantiation).

module Test

import std.core.print

@main
func main() -> lang.i64 {
    let a = [1, 2, 3];
    let b = [1, 2, 3];
    let c = [1, 2, 4];
    if not (a == b) { return 1 }
    if a == c { return 2 }

    let s = ["x", "y"];
    let t = ["x", "y"];
    let u = ["x", "z"];
    if not (s == t) { return 3 }
    if s == u { return 4 }

    let n1 = [[1, 2], [3]];
    let n2 = [[1, 2], [3]];
    let n3 = [[1, 2], [4]];
    if not (n1 == n2) { return 5 }
    if n1 == n3 { return 6 }

    0
}
