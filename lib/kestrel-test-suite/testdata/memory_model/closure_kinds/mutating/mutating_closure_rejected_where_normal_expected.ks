// test: diagnostics
// stdlib: false

// docs/design/closures.md passing table, row `mutating` → normal: ✗. Weakening
// an exclusive-call value to a shared, freely-copyable normal slot would let
// two aliases write through the same views, so it is a kind mismatch (E624).
module Test

func callTwice(f: (lang.i64) -> ()) {
    f(1);
    f(2);
}

func test() {
    var total = 0;
    var bump: mutating (lang.i64) -> () = { total = lang.i64_add(total, it); };
    callTwice(bump); // ERROR(E624)
}
