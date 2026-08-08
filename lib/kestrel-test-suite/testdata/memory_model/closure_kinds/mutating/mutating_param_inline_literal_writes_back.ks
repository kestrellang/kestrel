// test: execution
// stdlib: false
// expect-exit: 0

// Headline `mutating` behavior from docs/design/closures.md: a
// `mutating (Int64) -> ()` parameter gives the callee write-back into the
// caller's frame. An inline literal is built directly for the expected kind,
// so its assignments hit the enclosing `total` through &mutating views.
module Test

func applyTwice(mutating action: mutating (lang.i64) -> ()) {
    action(1);
    action(2);
}

@main
func main() -> lang.i64 {
    var total = 0;
    applyTwice({ total = lang.i64_add(total, it); });
    if lang.i64_eq(total, 3) { } else { return 1; }
    0
}
