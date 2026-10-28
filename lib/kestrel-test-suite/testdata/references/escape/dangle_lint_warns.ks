// test: diagnostics
// stdlib: true

// E504 dangle lint (stage 1.5 item 3): a returned ref fabricated from
// `Pointer(to:)` on a LOCAL of the same function points at storage that
// dies at return — every use is use-after-free. WARNING, not error:
// PointerDerived refs inherit the pointer's contract (references-gaps.md
// §10.3), so this claims only the provably-silly same-function shape.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

func dangleTail() -> &Int64 {
    var x = 42;
    Pointer(to: x).value // WARN: storage dies when the function returns
}

func dangleExplicitReturn() -> &mutating Int64 {
    var x = 7;
    return Pointer(to: x).mutatingValue // WARN: storage dies when the function returns
}

func dangleThroughLet() -> &Int64 {
    let y = 3;
    let p = Pointer(to: y);
    p.value // WARN: storage dies when the function returns
}
