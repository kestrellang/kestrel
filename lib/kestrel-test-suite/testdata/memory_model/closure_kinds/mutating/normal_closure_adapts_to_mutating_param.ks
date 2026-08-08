// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md passing table, row `normal` → `mutating`: ✓. A
// read-only body tolerates exclusive calling, so both an inline read-only
// literal and an existing normal closure VALUE pass into a `mutating`
// parameter; the adapter (not the `let` binding) supplies the mutable place.
module Test

func sumTwo(mutating f: mutating (lang.i64) -> lang.i64) -> lang.i64 {
    lang.i64_add(f(1), f(2))
}

@main
func main() -> lang.i64 {
    let base = 10;
    let a = sumTwo({ lang.i64_add(it, base) });
    if lang.i64_eq(a, 23) { } else { return 1; }

    let g: (lang.i64) -> lang.i64 = { lang.i64_add(it, 100) };
    let b = sumTwo(g);
    if lang.i64_eq(b, 203) { } else { return 2; }
    0
}
