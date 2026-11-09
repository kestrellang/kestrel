// test: diagnostics
// stdlib: true

// Compound assignment through a failable property reports only E214. The
// lent `Optional[&T]` read doesn't conform to AddAssign, but that E107 would
// just repeat the same problem, so inference stays quiet.

module Test

struct Box {
    var items: [Int64];
    var head?: Int64 {
        ref { if self.items.isEmpty { return null; } let r = &self.items(0); .Some(r) }
        mutating ref { if self.items.isEmpty { return null; } let r = &mutating self.items(0); .Some(r) }
    }
}

@main
func main() {
    var b = Box(items: [1]);
    b.head += 1; // ERROR: cannot assign to failable place 'head'
    var o: Int64? = .Some(1);
    o += 1; // ERROR: does not conform to protocol
}
