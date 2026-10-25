// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 NEGATIVE control — pins the `size <= 8` boundary of the single-field
// collapse in `classify_named`.
//
// `Optional[Int64]` is 16 bytes, so `Slot64` exceeds the collapse threshold and
// falls straight through to `TypeRepr::Aggregate` in BOTH backends — it always
// worked, before and after the fix. It guards the other side of the branch: a
// future change that widens the threshold (or removes the size test) would make
// this file start failing the way the size-8 shapes used to.
module Test

struct Slot64 {
    let o: std.result.Optional[std.numeric.Int64]
}

struct Holder {
    let s: Slot64
}

func value(s: Slot64) -> std.numeric.Int64 {
    match s.o {
        .Some(n) => n,
        .None => 999
    }
}

func makeSlot(n: std.numeric.Int64) -> Slot64 { Slot64(o: .Some(n)) }

@main
func main() -> lang.i64 {
    let a = Holder(s: makeSlot(11));
    let b = Holder(s: makeSlot(22));
    if value(a.s) != 11 { return 1 }
    if value(b.s) != 22 { return 2 }

    var xs = std.collections.Array[Slot64]();
    xs.append(makeSlot(33));
    xs.append(makeSlot(44));
    if value(xs(0)) != 33 { return 3 }
    if value(xs(1)) != 44 { return 4 }

    if value(Slot64(o: .None)) != 999 { return 5 }
    0
}
