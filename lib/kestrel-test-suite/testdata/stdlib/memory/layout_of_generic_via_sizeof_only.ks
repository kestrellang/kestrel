// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (fragility audit G2, Op half): a generic instantiation reachable
// ONLY as the type operand of `Op::SizeOf`/`Op::AlignOf` was never seeded into
// mono's concrete-type worklist, so no `MonoStruct` was built for it and both
// backends fell back to `classify_named`'s pointer-sized scalar — `sizeof`
// answered 8 for a 24-byte struct, silently.
//
// A NON-generic struct does not reproduce this: concrete functions are
// unconditionally mono roots, so their `init` seeds the type as a value type.
// The struct must be a generic instantiation whose `init` is never called.
//
// `Layout` is the allocator ABI (`std.memory.Allocator.allocate(layout:)`), so
// an under-reported size is a heap under-allocation, not a cosmetic number.

module Test

// Never constructed. `GhostG[Int64]` appears only inside the monomorphized
// `Layout.of[GhostG[Int64]]`, as `Op::SizeOf` / `Op::AlignOf` operands.
struct GhostG[T] {
    var a: T
    var b: T
    var c: T
}

@main
func main() -> lang.i64 {
    let ghost = std.memory.Layout.of[GhostG[Int64]]();
    if ghost.size != 24 { return 1 }
    if ghost.alignment != 8 { return 2 }

    // Control: same shape, non-generic, so it is seeded the ordinary way.
    // Both must agree — that they disagreed was the bug.
    let plain = std.memory.Layout.of[PlainBig]();
    if plain.size != ghost.size { return 3 }
    if plain.alignment != ghost.alignment { return 4 }
    0
}

struct PlainBig {
    var a: Int64
    var b: Int64
    var c: Int64
}
