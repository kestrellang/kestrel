// test: execution
// stdlib: true
// expect-exit: 0
//
// #146 (static-var facet): a protocol-extension default body that reads/writes a
// COMPUTED static property requirement via `Self.prop` must witness-dispatch to
// the conformer's accessor. `Self.prop` collapses to a bare reference to the
// requirement (receiver implicit), so a regression lowered it as a stored
// global → "global entity not found in statics" (read) / a static default whose
// `Self` use is body-only dropped its self_type (write).

module Test

protocol Scaled {
    static var factor: Int64 { get }
    func raw() -> Int64
}

extend Scaled {
    // reads a get-only computed static property via Self
    func scaled() -> Int64 { self.raw() * Self.factor }
}

struct Doubler: Scaled {
    var v: Int64;
    static var factor: Int64 { 2 } // computed (get-only)
    func raw() -> Int64 { self.v }
}

protocol Tally {
    static var total: Int64 { get set }
}

extend Tally {
    // a STATIC default whose only `Self` use is in the body (read + write)
    public static func add(n: Int64) { Self.total = Self.total + n; }
}

struct Counter: Tally {
    static var _t: Int64 = 0;
    static var total: Int64 {       // computed get/set over a stored backing var
        get { Counter._t }
        set { Counter._t = newValue }
    }
    public init() {}
}

@main
func main() -> lang.i32 {
    if Doubler(v: 5).scaled() != 10 { return 1 } // 5 * 2, computed-static read via Self
    Counter.add(2);                              // static default: read+write via Self
    Counter.add(3);
    if Counter.total != 5 { return 2 }
    0
}
