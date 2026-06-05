// test: execution
// stdlib: true

module Test

// Borrowing should NOT clone — refcount must be unchanged by the call.
func readRefCount(box: std.memory.RcBox[std.numeric.Int64]) -> std.numeric.Int64 {
    box.refCount()
}

@main
func main() -> lang.i64 {
    // --- 1. Assignment copies should clone (bump refcount) ---
    let a = std.memory.RcBox[std.numeric.Int64](42);
    if a.refCount() != 1 { return 1 }

    let b = a;
    if a.refCount() != 2 { return 2 }
    if b.refCount() != 2 { return 3 }
    if b.getValue() != 42 { return 4 }

    // --- 2. Multiple copies from same source stack refcounts ---
    let c = a;
    if a.refCount() != 3 { return 5 }
    if c.getValue() != 42 { return 6 }

    // --- 3. Borrowing function call does NOT clone ---
    let d = std.memory.RcBox[std.numeric.Int64](100);
    if d.refCount() != 1 { return 10 }
    let rc = readRefCount(d);
    if rc != 1 { return 11 }
    if d.refCount() != 1 { return 12 }

    // --- 4. Closure capture clones ---
    let e = std.memory.RcBox[std.numeric.Int64](200);
    if e.refCount() != 1 { return 20 }
    let getter = { e.getValue() };
    if e.refCount() != 2 { return 21 }
    if getter() != 200 { return 22 }

    // --- 5. Multiple closures capturing same value ---
    let f = { e.refCount() };
    if e.refCount() != 3 { return 30 }

    // --- 6. Copy chain: a -> b -> g ---
    let g = b;
    if a.refCount() != 4 { return 40 }
    if g.getValue() != 42 { return 41 }

    0
}
