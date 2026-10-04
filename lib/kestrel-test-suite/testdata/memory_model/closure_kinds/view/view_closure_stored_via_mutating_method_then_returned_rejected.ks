// test: diagnostics
// stdlib: true

// A view closure that captures a local, stored into a value by one of the
// value's `mutating` methods, after which the value is returned. The method
// itself is legal (the caller's frame outlives the call), but the caller's
// escape check never learns that `r` now carries a view of `k`, so `make()`
// returns a closure viewing its dead local. Found in the 2026-10 architecture
// review at 9767d2dc: `r.hs(0)(1)` printed 50221193882060886 instead of 6.
// The same shape is how Perch stores middleware (router.ks), so it matters
// in practice.
// EXPECTED TO FAIL until a `mutating` receiver picks up the provenance of the
// view-carrying arguments stored into it.
module Test

struct Reg {
    var hs: [(Int64) -> Int64];
    mutating func add(h: (Int64) -> Int64) { self.hs.append(h); }
}

func make() -> Reg {
    var r = Reg(hs: []);
    let k: Int64 = 5;
    r.add({ it + k });
    r // ERROR(E494)
}
