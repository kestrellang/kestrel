// test: diagnostics
// stdlib: false

// A callee may store a view closure argument into a `mutating` closure- or
// tuple-typed argument. The caller's variable then carries the view, so
// returning it is E494 (docs/design/closure-fields.md). Calling it locally is
// fine.
module Test

func put(mutating slot: () -> lang.i64, f: () -> lang.i64) { slot = f; }
func putT(mutating t: (() -> lang.i64, lang.i64), f: () -> lang.i64) { t.0 = f; }

func escapes() -> () -> lang.i64 {
    let k: lang.i64 = 5;
    var g: () -> lang.i64 = { 0 };
    put(g, { k });
    g // ERROR(E494)
}

func escapesInTuple() -> (() -> lang.i64, lang.i64) {
    let k: lang.i64 = 5;
    var t: (() -> lang.i64, lang.i64) = ({ 0 }, 1);
    putT(t, { k });
    t // ERROR(E494)
}

func local() -> lang.i64 {
    let k: lang.i64 = 5;
    var g: () -> lang.i64 = { 0 };
    put(g, { k });
    g()
}
