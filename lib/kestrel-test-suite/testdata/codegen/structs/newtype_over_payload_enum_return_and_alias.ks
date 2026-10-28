// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 regression — two live values returned by the SAME callee in one frame.
//
// This is the sharpest form of the aliasing symptom: if the newtype's "value"
// is really the callee's stack-slot address, both calls hand back the SAME
// address (the callee reuses its slot), so the second call retroactively
// rewrites the first result. A test that constructs and consumes one value per
// frame cannot see this — which is precisely why the bug shipped: both existing
// `IoError` tests only ever had one live instance.
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
        .A => -1,
        .Other(c) => std.numeric.Int64(from: c)
    }
}

func makeWrap(c: std.numeric.Int32) -> Wrap { Wrap(kind: .Other(code: c)) }

@main
func main() -> lang.i64 {
    let x = makeWrap(11);
    let y = makeWrap(22);
    let z = makeWrap(33);
    // Read x LAST: with slot aliasing the earlier bindings track the newest call.
    if code(z) != 33 { return 1 }
    if code(y) != 22 { return 2 }
    if code(x) != 11 { return 3 }

    // An assignment copy must be independent of its source.
    let copy = x;
    let w = makeWrap(44);
    if code(copy) != 11 { return 4 }
    if code(w) != 44 { return 5 }
    0
}
