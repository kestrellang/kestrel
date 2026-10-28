// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 control — pins that 2+-field structs are unaffected by the fix.
//
// `Pair` has the same aggregate-typed field as the F40 subject but a second
// field alongside it, so `is_single_field` is false and it was already
// classified `Aggregate` in both backends. The fix touches only the one-field
// arm; this file is the "nothing else moved" witness beside
// `newtype_over_payload_enum_struct_field.ks`.
module Test

enum Kind {
    case A
    case Other(code: std.numeric.Int32)
}

struct Pair {
    let kind: Kind
    let tag: std.numeric.Int32
}

func code(p: Pair) -> std.numeric.Int64 {
    match p.kind {
        .A => 777,
        .Other(c) => std.numeric.Int64(from: c)
    }
}

func makePair(c: std.numeric.Int32, tag: std.numeric.Int32) -> Pair {
    Pair(kind: .Other(code: c), tag: tag)
}

@main
func main() -> lang.i64 {
    var xs = std.collections.Array[Pair]();
    xs.append(makePair(11, 1));
    xs.append(makePair(22, 2));
    if code(xs(0)) != 11 { return 1 }
    if code(xs(1)) != 22 { return 2 }

    let tag0: std.numeric.Int32 = 1;
    let tag1: std.numeric.Int32 = 2;
    if xs(0).tag != tag0 { return 3 }
    if xs(1).tag != tag1 { return 4 }

    let a = makePair(33, 3);
    let b = makePair(44, 4);
    if code(a) != 33 { return 5 }
    if code(b) != 44 { return 6 }

    let unit = Pair(kind: .A, tag: 9);
    if code(unit) != 777 { return 7 }
    0
}
