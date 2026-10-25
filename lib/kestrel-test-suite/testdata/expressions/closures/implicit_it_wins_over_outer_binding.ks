// test: execution
// stdlib: true
// expect-exit: 0

// Runtime half of implicit_it_shadows_outer_binding_warns.ks: with an outer
// `let it`, `{ it + 1 }` still adds to the closure's own parameter (the
// element), not to the outer 100. The compiler warns (E143); warnings do not
// fail an execution test.

module Test

@main
func main() -> lang.i64 {
    let it = 100;
    let xs = [1, 2];
    let r = xs.map { it + 1 };
    if r(0) != 2 { return 1; }
    if r(1) != 3 { return 2; }
    if it != 100 { return 3; }
    0
}
