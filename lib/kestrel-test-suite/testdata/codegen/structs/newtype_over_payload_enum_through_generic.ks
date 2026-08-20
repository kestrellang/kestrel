// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 regression — newtype-over-aggregate passed through a generic identity.
//
// `ident[T]` monomorphizes to a by-value pass of `Wrap`. The ABI decision is
// made from `TypeRepr` alone (`param_pass_mode` / `return_mode`): with the
// misclassification the parameter was passed ByVal as an I64 holding the
// caller's stack-slot address, and returned Direct — so the callee handed back
// a pointer into a frame that was about to die.
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

func ident[T](v: T) -> T { v }

func makeWrap(c: std.numeric.Int32) -> Wrap { Wrap(kind: .Other(code: c)) }

@main
func main() -> lang.i64 {
    if code(ident(makeWrap(33))) != 33 { return 1 }
    // Two round-trips live at once.
    let a = ident(makeWrap(11));
    let b = ident(makeWrap(22));
    if code(a) != 11 { return 2 }
    if code(b) != 22 { return 3 }
    // Nested instantiation.
    if code(ident(ident(makeWrap(44)))) != 44 { return 4 }
    // The generic must still work for a plain scalar.
    let n: std.numeric.Int64 = ident(7);
    if n != 7 { return 5 }
    0
}
