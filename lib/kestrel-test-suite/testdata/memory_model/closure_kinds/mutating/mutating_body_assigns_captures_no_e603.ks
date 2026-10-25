// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md Diagnostics table: E603 ("cannot assign to a
// capture") is kept for normal bodies but LIFTED when the expected kind is
// `mutating`. A body assigning to several captures — including the narrow
// place `c.n` rather than all of `c` — compiles and writes back.
module Test

struct Counter { var n: lang.i64 }

func runTwice(mutating action: mutating () -> ()) {
    action();
    action();
}

@main
func main() -> lang.i64 {
    var c = Counter(n: 0);
    var flag = 0;
    runTwice({ c.n = lang.i64_add(c.n, 5); flag = 1; });
    if lang.i64_eq(c.n, 10) { } else { return 1; }
    if lang.i64_eq(flag, 1) { } else { return 2; }
    0
}
