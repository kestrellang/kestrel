// test: execution
// stdlib: true
// backends: cranelift,llvm

// Evaluation-order pin: in `x(at: i) = rhs`, the RHS is evaluated BEFORE
// the target's place is fabricated (receiver + index lowering) — matching
// the shipped assignment-through-&mutating-call order. The RHS mutates
// the index variable, so RHS-first stores at the NEW index.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Buf {
    var p: Pointer[Int64]
    subscript(at index: Int64) -> Int64 {
        ref { unsafe self.p.offset(by: index).value }
        mutating ref { unsafe self.p.offset(by: index).mutatingValue }
    }
}

func rhsBump(k: Pointer[Int64]) -> Int64 {
    unsafe k.write(1);
    42
}

@main
func main() -> lang.i64 {
    let p = SystemAllocator()
        .allocate(Layout.array[Int64](2))
        .unwrap()
        .cast[Int64]();
    unsafe p.write(10);
    unsafe p.offset(by: 1).write(20);
    let k = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    unsafe k.write(0);

    var b = Buf(p: p);
    // RHS-first: rhsBump sets k to 1, THEN the index reads k (= 1).
    b(at: unsafe k.read()) = rhsBump(k);
    if unsafe p.read() != 10 { return 1; }
    if unsafe p.offset(by: 1).read() != 42 { return 2; }
    0
}
