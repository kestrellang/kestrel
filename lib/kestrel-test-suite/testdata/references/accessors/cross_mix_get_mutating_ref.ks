// test: execution
// stdlib: true
// backends: cranelift,llvm

// Cross-mix `get` + `mutating ref` (the Swift get+_modify shape): reads
// go through `get` (which CLAMPS), but RMW goes through the mutating ref
// and BYPASSES get's normalization. This divergence is the documented
// coherence contract — intended behavior, not a bug. The accessor pair
// must agree on the place they describe; get may present a normalized
// VIEW of it.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Clamped {
    var p: Pointer[Int64]

    subscript(at index: Int64) -> Int64 {
        get {
            let raw = self.p.offset(by: index).value;
            if raw > 100 { 100 } else { raw }
        }
        mutating ref { self.p.offset(by: index).mutatingValue }
    }
}

@main
func main() -> lang.i64 {
    let p = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    p.write(99);
    var c = Clamped(p: p);

    if c(at: 0) != 99 { return 1; }

    // RMW through the mutating ref: the += reads the RAW storage (99),
    // not get's clamped view — raw becomes 104.
    c(at: 0) += 5;
    if p.read() != 104 { return 2; }

    // Reads still clamp.
    if c(at: 0) != 100 { return 3; }

    // Plain assignment also goes through the mutating ref (raw store).
    c(at: 0) = 250;
    if p.read() != 250 { return 4; }
    if c(at: 0) != 100 { return 5; }
    0
}
