// test: diagnostics
// stdlib: false

// docs/design/closures.md passing table, row `mutating` → `escaping`: ✗
// "frame-bound". Nothing frame-bound ever flows into an `escaping` slot — a
// view kind cannot acquire an owned environment by coercion — so this is a
// kind mismatch (E624) at the argument, not an escape (E494) at a return.
module Test

func store(f: escaping () -> ()) { }

func test() {
    var total = 0;
    var bump: mutating () -> () = { total = lang.i64_add(total, 1); };
    store(bump); // ERROR(E624)
}
