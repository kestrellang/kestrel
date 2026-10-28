// test: execution
// stdlib: true
// expect-exit: 0
//
// #187 (BUG-51): the binding must extract from the alternative that actually
// matched — its access path differs per alternative. Here `n` is field 0 of
// `.A` but field 1 of `.B`; the leaf reached via `.B` must read field 1, not
// `.A`'s field 0. Pins both the unified-local fix (no undefined-local ICE) and
// the per-leaf binding-path selection.

module Test

enum E {
    case A(Int64, Bool)
    case B(Bool, Int64)
}

func pick(e: E) -> Int64 {
    match e {
        .A(n, _) or .B(_, n) => n
    }
}

@main
func main() -> lang.i32 {
    if pick(.A(3, true)) != 3 { return 1 }
    if pick(.B(false, 7)) != 7 { return 2 }
    0
}
