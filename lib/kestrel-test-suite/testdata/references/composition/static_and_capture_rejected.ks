// test: diagnostics
// stdlib: true

// References 2b containment: statics and closure captures reject
// ref-bearing (non-Static) types — the 2a gates, now live.
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
    consume { o.isSome() } // ERROR(E212)
}
