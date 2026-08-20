// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 regression — a newtype over an AGGREGATE field must be carried by
// address, not collapsed to an integer scalar.
//
// `Wrap` has exactly one field whose type is a payload-carrying enum (layout
// size 8, align 4 => TypeRepr::Aggregate). Cranelift's `classify_named`
// flattened its `if let` chain with `&&`, so a non-Scalar field repr fell
// through to the integer-by-size mapping and answered `Scalar(I64)`.
// `compile_struct` then returned the field's stack-slot ADDRESS as the
// "value", and every by-repr copy moved 8 bytes of pointer instead of the
// enum's bytes. LLVM nested the `if let` and returned `Aggregate` — the two
// backends disagreed, silently, on the default backend.
//
// Here `Wrap` is stored as a field of a `Holder` built in a callee: reading
// `h.w` back after the constructing frame returned observed the stale slot.
module Test

enum Kind {
    case A
    case Other(code: std.numeric.Int32)
}

struct Wrap {
    let kind: Kind
}

struct Holder {
    let w: Wrap
    let tag: std.numeric.Int64
}

func code(w: Wrap) -> std.numeric.Int64 {
    match w.kind {
        .A => 777,
        .Other(c) => std.numeric.Int64(from: c)
    }
}

func makeWrap(c: std.numeric.Int32) -> Wrap { Wrap(kind: .Other(code: c)) }
func makeHolder(c: std.numeric.Int32, tag: std.numeric.Int64) -> Holder {
    Holder(w: makeWrap(c), tag: tag)
}

@main
func main() -> lang.i64 {
    // Two holders live at once: with the address-as-value bug both `w` fields
    // aliased the same reused stack slot and reported the last value written.
    let h1 = makeHolder(11, 1);
    let h2 = makeHolder(22, 2);
    if code(h1.w) != 11 { return 1 }
    if code(h2.w) != 22 { return 2 }
    if h1.tag != 1 { return 3 }
    if h2.tag != 2 { return 4 }

    // Payload-less case through the same shape.
    let h3 = Holder(w: Wrap(kind: .A), tag: 3);
    if code(h3.w) != 777 { return 5 }
    0
}
