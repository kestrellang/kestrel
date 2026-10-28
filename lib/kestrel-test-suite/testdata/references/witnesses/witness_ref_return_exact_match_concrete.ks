// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: a `-> &Item` protocol requirement is satisfied by an impl with
// the exact same ref shape — conformance checking compares the ref returns
// faithfully (no E458), and the concrete (stage-1 direct dispatch) path
// reads and writes through the returned refs.
module Test

protocol RefGetter {
    type Item
    func fetch() -> &Item
}

protocol MutGetter {
    type Item
    mutating func access() -> &mutating Item
}

struct H: RefGetter {
    type Item = Int64
    var v: Int64

    func fetch() -> &Int64 {
        self.v
    }
}

struct M: MutGetter {
    type Item = Int64
    var v: Int64

    mutating func access() -> &mutating Int64 {
        self.v
    }
}

@main
func main() -> lang.i64 {
    let h = H(v: 5);
    if h.fetch() != 5 { return 1; }
    var m = M(v: 7);
    m.access() = 9;
    if m.v != 9 { return 2; }
    0
}
