// test: execution
// stdlib: true
// backends: cranelift,llvm

// Stage 0.5 Pointer bridge: `Pointer(to: x)` captures the address of the
// borrowed place, and `read()` round-trips the value (docs/plans/references/
// stage0.5/tests.md). Scalar pointees ride the @guaranteed borrow chain —
// this is the path the `codegen_byref_scalar_deref` bug class lives on.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    let x: Int64 = 42;
    let p = Pointer(to: x);
    if p.read() != 42 { return 1; }

    // A second capture of the same place reads the same value.
    let q = Pointer(to: x);
    if q.read() != 42 { return 2; }
    0
}
