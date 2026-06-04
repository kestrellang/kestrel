// test: execution
// stdlib: false

// Regression: a generic subscript with a where-clause and both `get` and
// `set` blocks must dispatch the setter call correctly. The setter's MIR
// signature inherits its parent subscript's type parameters; without that
// inheritance, the call at `b(at: Slot0()) = v` forwards 1 type arg to a
// 0-type-param setter and the monomorphizer rejects it.

module Test

protocol Indexer {
    type Output
    func load(box box: Box) -> Output
    func store(mutating box box: Box, value value: Output)
}

struct Box {
    var v0: lang.i64
    var v1: lang.i64

    public init(v0 v0: lang.i64, v1 v1: lang.i64) {
        self.v0 = v0;
        self.v1 = v1
    }

    public func loadV0() -> lang.i64 { self.v0 }
    public func loadV1() -> lang.i64 { self.v1 }
    public mutating func storeV0(value x: lang.i64) { self.v0 = x }
    public mutating func storeV1(value x: lang.i64) { self.v1 = x }

    public subscript[I](at i: I) -> I.Output where I: Indexer {
        get { i.load(box: self) }
        set { i.store(box: self, value: newValue) }
    }
}

struct Slot0 { public init() {} }
struct Slot1 { public init() {} }

extend Slot0: Indexer {
    type Indexer.Output = lang.i64
    public func load(box box: Box) -> lang.i64 { box.loadV0() }
    public func store(mutating box box: Box, value value: lang.i64) {
        box.storeV0(value: value)
    }
}

extend Slot1: Indexer {
    type Indexer.Output = lang.i64
    public func load(box box: Box) -> lang.i64 { box.loadV1() }
    public func store(mutating box box: Box, value value: lang.i64) {
        box.storeV1(value: value)
    }
}

@main
func main() -> lang.i64 {
    var b = Box(v0: 10, v1: 20);
    if lang.i64_signed_lt(b(at: Slot0()), 10) { return 1 }
    if lang.i64_signed_lt(b(at: Slot1()), 20) { return 2 }
    b(at: Slot0()) = 99;
    b(at: Slot1()) = 88;
    if lang.i64_signed_lt(b.loadV0(), 99) { return 3 }
    if lang.i64_signed_lt(b.loadV1(), 88) { return 4 }
    0
}
