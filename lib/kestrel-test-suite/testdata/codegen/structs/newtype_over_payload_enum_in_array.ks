// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 regression — newtype-over-aggregate round-tripped through an Array.
//
// The array element path forces the value into HEAP storage (not just a stack
// slot), so a `Scalar(I64)` misclassification stores 8 bytes of stack ADDRESS
// into the buffer. Every element then points at the same (dead) frame slot:
// before the fix both reads answered the payload of the last construction.
module Test

enum Kind {
    case A
    case Other(code: std.numeric.Int32)
}

struct Wrap {
    let kind: Kind
}

func code(w: Wrap) -> std.numeric.Int64 {
    match w.kind {
        .A => 777,
        .Other(c) => std.numeric.Int64(from: c)
    }
}

func makeWrap(c: std.numeric.Int32) -> Wrap { Wrap(kind: .Other(code: c)) }

@main
func main() -> lang.i64 {
    var xs = std.collections.Array[Wrap]();
    xs.append(makeWrap(11));
    xs.append(makeWrap(22));
    xs.append(Wrap(kind: .A));
    if xs.count != 3 { return 1 }
    if code(xs(0)) != 11 { return 2 }
    if code(xs(1)) != 22 { return 3 }
    if code(xs(2)) != 777 { return 4 }

    // Read back again after growth, in case a realloc moved the buffer.
    xs.append(makeWrap(33));
    if code(xs(0)) != 11 { return 5 }
    if code(xs(3)) != 33 { return 6 }
    0
}
