// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 regression — newtype-over-aggregate as an `Optional` payload.
//
// `Optional[Wrap]` stores the payload by the element's `TypeRepr`. With `Wrap`
// misclassified as `Scalar(I64)` the enum payload slot held a stack address,
// so the value bound out by the `.Some` arm was garbage (cranelift reported the
// `.A` arm's 777 for a `.Other(44)` payload).
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
    let o: std.result.Optional[Wrap] = .Some(makeWrap(44));
    match o {
        .Some(w) => { if code(w) != 44 { return 1 } },
        .None => return 2
    };

    // Two optionals live at once.
    let a: std.result.Optional[Wrap] = .Some(makeWrap(11));
    let b: std.result.Optional[Wrap] = .Some(makeWrap(22));
    match a {
        .Some(w) => { if code(w) != 11 { return 3 } },
        .None => return 4
    };
    match b {
        .Some(w) => { if code(w) != 22 { return 5 } },
        .None => return 6
    };

    let n: std.result.Optional[Wrap] = .None;
    match n {
        .Some(_) => return 7,
        .None => 0
    }
}
