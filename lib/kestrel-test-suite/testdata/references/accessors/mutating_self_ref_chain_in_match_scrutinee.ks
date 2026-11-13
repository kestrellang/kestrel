// test: execution
// stdlib: true

// A builder chain of two or more links returning `&mutating Self` on a
// temporary, used directly as a `match` scrutinee. The chain's borrow of the
// temporary only needs to last until the final call returns an owned value,
// so it shouldn't stay live into the arms.
//
// Known gap: reports E497 ("a reference cannot stay live across a
// control-flow merge"). A single link works, as does binding the result
// with `let` first or using `if let`.

module Test

struct Builder: not Copyable {
    var parts: Array[String]

    init(first: String) { self.parts = [first]; }

    mutating func add(part: String) -> &mutating Builder {
        self.parts.append(part);
        self
    }

    func finish() -> Optional[Int64] { .Some(self.parts.count) }
}

@main
func main() -> lang.i64 {
    match Builder("a").add("b").add("c").finish() {
        .Some(n) => { if n != 3 { return 1 } },
        .None => { return 2 }
    }
    0
}
