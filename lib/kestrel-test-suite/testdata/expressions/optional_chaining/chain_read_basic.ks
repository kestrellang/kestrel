// test: execution
// stdlib: true
// expect-exit: 0

// Reading through `?.`: fields, methods, multi-level chains, promotion of a
// non-optional tail, flattening of an optional tail, and `??` fallback.

module Test

struct Inner {
    var n: Int64
    var maybe: Int64?
    func doubled() -> Int64 { self.n * 2 }
}

struct Outer {
    var inner: Inner?
    var label: Int64
}

@main
func main() -> lang.i64 {
    let full: Inner? = .Some(Inner(n: 4, maybe: .Some(9)));
    let none: Inner? = .None;

    // field
    let a: Int64? = full?.n;
    if a != .Some(4) { return 1 }
    if none?.n != .None { return 2 }

    // method
    if full?.doubled() != .Some(8) { return 3 }
    if none?.doubled() != .None { return 4 }

    // promotion without an annotation
    let b = full?.n;
    if (b ?? 0) != 4 { return 5 }

    // flattening: `maybe` is already `Int64?`
    let c: Int64? = full?.maybe;
    if c != .Some(9) { return 6 }

    // multi-level
    let outer: Outer? = .Some(Outer(inner: full, label: 1));
    if outer?.inner?.n != .Some(4) { return 7 }
    let hollow: Outer? = .Some(Outer(inner: .None, label: 2));
    if hollow?.inner?.n != .None { return 8 }

    // non-optional step after a bind
    if outer?.label != .Some(1) { return 9 }

    // `??` fallback
    if (none?.n ?? 7) != 7 { return 10 }
    0
}
