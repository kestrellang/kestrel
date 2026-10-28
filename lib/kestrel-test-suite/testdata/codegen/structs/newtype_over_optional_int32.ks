// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 regression — the SHIPPED shape: a one-field struct over `Optional[T]`
// at a small `T`.
//
// `Optional[Int32]` lays out as 8 bytes (i8 tag + i32 payload) => Aggregate, so
// `Slot` is a newtype over an aggregate and hit the same misclassification.
// This is exactly the shape of the stdlib's `OptionalIterator[T]` /
// `ResultIterator[T]` at small `T`, which is why the bug was reachable from
// ordinary library code and not just from hand-written newtypes.
//
// Before the fix cranelift answered 999 (the `.None` arm) for every read and
// crashed with SIGBUS on the array path.
module Test

struct Slot {
    let o: std.result.Optional[std.numeric.Int32]
}

struct Holder {
    let s: Slot
    let tag: std.numeric.Int64
}

func value(s: Slot) -> std.numeric.Int64 {
    match s.o {
        .Some(n) => std.numeric.Int64(from: n),
        .None => 999
    }
}

func makeSlot(n: std.numeric.Int32) -> Slot { Slot(o: .Some(n)) }

@main
func main() -> lang.i64 {
    // As a struct field, two live at once.
    let h1 = Holder(s: makeSlot(11), tag: 1);
    let h2 = Holder(s: makeSlot(22), tag: 2);
    if value(h1.s) != 11 { return 1 }
    if value(h2.s) != 22 { return 2 }

    // In an array (heap storage).
    var xs = std.collections.Array[Slot]();
    xs.append(makeSlot(33));
    xs.append(makeSlot(44));
    if value(xs(0)) != 33 { return 3 }
    if value(xs(1)) != 44 { return 4 }

    // The `.None` payload must still read as absent.
    let empty = Slot(o: .None);
    if value(empty) != 999 { return 5 }
    0
}
