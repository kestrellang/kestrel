// test: diagnostics
// stdlib: true

// References 2b containment: a STATIC declaration rejects ref-bearing
// (non-Static) types — the 2a gate, still live (E505).
//
// The closure half of this file is gone: E212 ("closures cannot capture
// non-Static bindings") RETIRED with the closure-kinds work
// (docs/design/closures.md, Diagnostics table; plan lockstep 6). A view-kind
// closure's env is frame-bound, so capturing a ref-bearing value is sound; the
// call below is now legal and is left in place to pin that.
module Test

struct Holder {
    static var cache: Optional[&Int64] = .None // ERROR(E505)
}

func consume(f: () -> Bool) -> Bool {
    f()
}

func bad() -> Bool {
    var x = 1;
    let r = &x;
    let o: Optional[&Int64] = .Some(r);
    consume { o.isSome() }
}
