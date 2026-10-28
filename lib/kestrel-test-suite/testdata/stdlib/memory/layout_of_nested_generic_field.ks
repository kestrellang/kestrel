// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (fragility audit G2, field half): mono's type collector recurses
// into type args, `Pointer`, `Ref` and `Tuple` — but never into a struct's
// FIELD types. And the layout fixed-point loop iterated its worklist by shared
// borrow, so it structurally could not add a type it discovered while walking
// fields.
//
// Consequence: `Inner[Int64]` is reachable only as a field type of
// `Outer[Int64]`. It was never seeded, `mono_size_and_align` returned `None`,
// `all_resolved` went false, and the CONTAINING struct `Outer[Int64]` was
// dropped from `mono_structs` too — with no diagnostic. Both backends then
// fell back to a pointer-sized scalar: 8 instead of 32.
//
// Execution-kind on purpose: it also exercises `struct_field_offset` and
// `classify_named`, which returned a bare `0` offset and the container type as
// the field type under the fallback.

module Test

struct Inner[T] {
    var a: T
    var b: T
    var c: T
}

// Both generic, so neither `init` is monomorphized at [Int64] — nothing ever
// materializes an `Inner[Int64]` VALUE.
struct Outer[U] {
    var i: Inner[U]
    var x: U
}

// Concrete, so it is a mono root: its parameter seeds `Outer[Int64]` as a value
// type. `Inner[Int64]` still is not.
func peek(o o: Outer[Int64]) -> Int64 {
    o.x
}

@main
func main() -> lang.i64 {
    let outer = std.memory.Layout.of[Outer[Int64]]();
    if outer.size != 32 { return 1 }
    if outer.alignment != 8 { return 2 }

    // Control: the same four words, flattened so no nested field type exists.
    // This printed the right answer even with the bug, isolating the loss.
    let flat = std.memory.Layout.of[Flat[Int64]]();
    if flat.size != outer.size { return 3 }
    if flat.alignment != outer.alignment { return 4 }
    0
}

struct Flat[U] {
    var a: U
    var b: U
    var c: U
    var x: U
}
