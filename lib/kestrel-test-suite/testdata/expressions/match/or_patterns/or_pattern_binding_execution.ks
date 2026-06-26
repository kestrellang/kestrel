// test: execution
// stdlib: true
// expect-exit: 0
//
// #187 (BUG-51): an or-pattern with a binding (`.A(x) or .B(x) => x`) used to
// ICE in OSSA verify ("operand used but never defined"). Each alternative
// lowered `x` to a DISTINCT local; the arm body read whichever the last
// alternative defined, while each match leaf bound its own — so the body read
// an unpopulated local. Existing coverage was diagnostics-only and never
// reached MIR. This runs it and checks both alternatives bind correctly.

module Test

enum E {
    case A(Int64)
    case B(Int64)
}

func pick(e: E) -> Int64 {
    match e {
        .A(x) or .B(x) => x
    }
}

@main
func main() -> lang.i32 {
    if pick(.A(3)) != 3 { return 1 }
    if pick(.B(7)) != 7 { return 2 }
    0
}
