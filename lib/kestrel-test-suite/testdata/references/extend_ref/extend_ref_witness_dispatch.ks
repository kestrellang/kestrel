// test: execution
// stdlib: true
// backends: cranelift,llvm

// End-to-end witness dispatch through `extend &T: P where T: P`: generic
// code bounded `X: Probe` instantiated at X = &Int64 resolves the ref
// extension's witness (the mono match_pattern Ref arm), whose body
// forwards to the POINTEE's witness through the transparent-place peel.
// Pins the Equatable SHAPE too (`other: Self` — a ref-typed param at
// borrow convention, the pointer ABI the stdlib comparison extensions
// ride), and the `&mutating` twin. A mis-resolved Self in the extension
// bodies would dispatch back onto the extension and recurse — loud
// failure, not silent corruption.
module Test

protocol Probe { func probe() -> Int64 }
protocol Eq2 { func eq2(other: Self) -> Bool }

extend Int64: Probe {
    public func probe() -> Int64 { 7 }
}
extend Int64: Eq2 {
    public func eq2(other: Self) -> Bool { self == other }
}

extend &T: Probe where T: Probe {
    public func probe() -> Int64 { self.probe() }
}
extend &mutating T: Probe where T: Probe {
    public func probe() -> Int64 { self.probe() }
}
extend &T: Eq2 where T: Eq2 {
    public func eq2(other: Self) -> Bool { self.eq2(other) }
}

func probeOf[X](x: X) -> Int64 where X: not Static, X: Probe { x.probe() }
func bothEq[X](a: X, b: X) -> Bool where X: not Static, X: Eq2 { a.eq2(b) }

@main
func main() -> Int64 {
    var v = 41;
    let r = &v;
    if probeOf[&Int64](r) != 7 { return 1; }

    var w = 42;
    let m = &mutating w;
    if probeOf[&mutating Int64](m) != 7 { return 2; }

    var a = 5;
    var b = 5;
    var c = 6;
    let ra = &a;
    let rb = &b;
    let rc = &c;
    if not bothEq[&Int64](ra, rb) { return 3; }
    if bothEq[&Int64](ra, rc) { return 4; }
    0
}
