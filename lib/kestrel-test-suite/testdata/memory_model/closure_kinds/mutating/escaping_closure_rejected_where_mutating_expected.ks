// test: diagnostics
// stdlib: false

// docs/design/closures.md passing table, row `escaping` → `mutating`: ✗ —
// "shared, not exclusive". An escaping closure is duplicable and its copies
// share one environment, so it can never supply the exclusive access a
// `mutating` slot demands (E624), even though it passes as normal/consuming.
module Test

func each(mutating action: mutating (lang.i64) -> ()) {
    action(1);
}

func test() {
    var seen = 0;
    var tally: escaping (lang.i64) -> () = { seen = lang.i64_add(seen, it); };
    each(tally); // ERROR(E624)
}
