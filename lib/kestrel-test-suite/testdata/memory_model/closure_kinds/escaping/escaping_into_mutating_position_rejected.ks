// test: diagnostics
// stdlib: false

// docs/design/closures.md — the passing table: escaping -> mutating is
// "✗ shared, not exclusive". An escaping closure's calls are shared/multi-handle,
// so it can never supply the exclusive-call capability a `mutating` closure
// parameter demands, even though it owns its environment.
module Test

func each(mutating f: mutating () -> ()) { f(); }

func makeSink() -> escaping () -> () {
    var n: lang.i64 = 0;
    { n = 1; }                 // escaping bodies may assign their own captures
}

func test() {
    var g = makeSink();
    each(g); // ERROR(E624)
}
