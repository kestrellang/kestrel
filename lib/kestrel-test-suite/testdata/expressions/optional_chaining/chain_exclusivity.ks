// test: diagnostics
// stdlib: true

module Test

struct Bag {
    var items: [Int64]
    var size: Int64
    mutating func add(n: Int64) { self.size = self.size + n; }
}

extend Optional[T] {
    func count() -> Int64 { if self.isSome() { 1 } else { 0 } }
}

func test() {
    var b: Bag? = .Some(Bag(items: [], size: 0));
    // The bind lends `b` mutably while the argument reads it. There is no
    // exclusivity check yet, so this compiles today.
    b?.add(b.count()); // ERROR
}
