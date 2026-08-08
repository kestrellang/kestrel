// test: diagnostics
// stdlib: true

// A lazy builder's parameter is `escaping`, and an escaping literal captures by
// ownership: a non-Copyable place owned by the frame is MOVED into the shared
// environment, leaving the source dead (docs/design/closures.md, "How it is
// captured", row 3). Later use is an ordinary use-after-move.
module Test

struct Token: not Copyable {
    var id: Int64

    // Calling a method widens the capture to the whole receiver, so the
    // closure owns `Token` itself rather than just the `id` projection.
    func matches(x: Int64) -> Bool { x == self.id }
}

func movedIntoMap() -> Int64 {
    let token = Token(id: 2);
    let mapped = [1, 2, 3].iter().map(as: { (x) in if token.matches(x) { 1 } else { 0 } });
    let out: Array[Int64] = mapped.collect();
    token.id // ERROR(E500)
}

func movedIntoFilter() -> Int64 {
    let token = Token(id: 3);
    let filtered = [1, 2, 3].iter().filter(where: { (x) in token.matches(x) });
    let out: Array[Int64] = filtered.collect();
    token.id // ERROR(E500)
}

func movedIntoSplitWhere() -> Int64 {
    let token = Token(id: 1);
    let arr = [1, 2, 1, 3];
    let segments = arr.split(where: { (x) in token.matches(x) });
    let count = segments.count;
    token.id // ERROR(E500)
}
