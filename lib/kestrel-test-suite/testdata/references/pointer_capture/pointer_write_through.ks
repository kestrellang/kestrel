// test: execution
// stdlib: true
// backends: cranelift,llvm

// Stage 0.5 Pointer bridge: the sole capture init is write-capable —
// `Pointer(to: x).write(v)` mutates the original `var` place (no
// `mutating:` twin exists; references-gaps.md §10.2 revised). If the
// capture took the address of a temporary copy instead of the place,
// the write would be lost and `x` would still read 5.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    var x: Int64 = 5;
    // `var` only because the `pointee` setter below needs a mutable
    // receiver binding; the capture itself works from a `let`.
    var p = Pointer(to: x);
    p.write(100);
    if x != 100 { return 1; }

    // Writing through `pointee` hits the same place.
    p.pointee = 7;
    if x != 7 { return 2; }
    0
}
