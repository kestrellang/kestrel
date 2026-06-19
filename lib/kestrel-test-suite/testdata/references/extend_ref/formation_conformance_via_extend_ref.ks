// test: diagnostics
// stdlib: true

// A ref TYPE ARGUMENT conforms to exactly what `extend &T: P` declares:
// `&Int64: Probe` holds because the extension declares it AND its
// `where T: Probe` bound holds at the pointee. Exercised through explicit
// generic-call instantiation (the call-site where-clause obligation —
// struct ANNOTATION formation only checks Static today, the G4 family,
// so it cannot anchor these). Type-level only: no witness is dispatched
// (that lands with witness lowering). Pins: (1) the accept side via the
// synthetic lang.& entity, (2) pointee bound failure still rejects,
// (3) undeclared protocols still reject, (4) NO `&mutating` ← `&`
// subsumption — each mutability conforms only via its own extension.
module Test

protocol Probe { func probe() -> Int64 }
protocol Other { func other() -> Int64 }

extend Int64: Probe {
    public func probe() -> Int64 { 3 }
}

extend &T: Probe where T: Probe {
    public func probe() -> Int64 { self.probe() }
}

func need[X]() -> Int64 where X: not Static, X: Probe { 1 }
func needOther[X]() -> Int64 where X: not Static, X: Other { 2 }

@main
func main() {
    let a = need[&Int64]();
    let b = needOther[&Int64](); // ERROR: !: Other
    let c = need[&Float64](); // ERROR: !: Probe
    let d = need[&mutating Int64](); // ERROR: !: Probe
}
