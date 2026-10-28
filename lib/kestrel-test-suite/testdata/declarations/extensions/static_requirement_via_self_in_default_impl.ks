// test: execution
// stdlib: true
// expect-exit: 0
//
// #146: a protocol-extension default body that calls a STATIC requirement via
// `Self.staticReq()` must dispatch to the conformer's witness at mono. The
// `Self.factor` access collapses to a bare reference to the requirement (the
// receiver is implicit), so the witness self_type must be derived as the
// protocol's `Self` (substituted to the conformer), not the requirement's
// function type — a regression there left the witness unresolved post-mono
// ("type 'FuncThick…' does not implement 'factor'").

module Test

protocol Scaled {
    static func factor() -> Int64
    func raw() -> Int64
}

extend Scaled {
    func scaled() -> Int64 { self.raw() * Self.factor() }
}

struct Doubler {
    var v: Int64;
}
extend Doubler: Scaled {
    public static func factor() -> Int64 { 2 }
    public func raw() -> Int64 { self.v }
}

struct Tripler {
    var v: Int64;
}
extend Tripler: Scaled {
    public static func factor() -> Int64 { 3 }
    public func raw() -> Int64 { self.v }
}

@main
func main() -> lang.i32 {
    if Doubler(v: 5).scaled() != 10 { return 1 } // 5 * 2
    if Tripler(v: 5).scaled() != 15 { return 2 } // 5 * 3 (distinct witness)
    0
}
