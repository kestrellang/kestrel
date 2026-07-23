// test: execution
// stdlib: true

// Set.remove(element:) returns the STORED element (T?) instead of Bool —
// matters when equality is coarser than identity.

module Test

// Equality/hash only look at `id`, so two Wrapper values with the same id
// are "equal" but distinguishable via `tag`.
struct Wrapper: Hashable {
    var id: Int64
    var tag: Int64

    public func isEqual(to other: Wrapper) -> Bool {
        self.id == other.id
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.id.hash(into: hasher)
    }
}

@main
func main() -> lang.i64 {
    var s = std.collections.Set[std.numeric.Int64]();
    s.insert(10);
    s.insert(20);

    // Present element: returns .Some(stored).
    match s.remove(10) {
        .Some(v) => { if v != 10 { return 1 } },
        .None => { return 2 }
    }
    if s.count != 1 { return 3 }

    // Absent element: returns .None.
    if s.remove(99).isSome() { return 4 }
    if s.count != 1 { return 5 }

    // Coarse equality: the stored value (tag 1) comes back, not the probe
    // (tag 2).
    var ws = std.collections.Set[Wrapper]();
    ws.insert(Wrapper(id: 7, tag: 1));
    match ws.remove(Wrapper(id: 7, tag: 2)) {
        .Some(w) => { if w.tag != 1 { return 6 } },
        .None => { return 7 }
    }
    if ws.count != 0 { return 8 }

    0
}
