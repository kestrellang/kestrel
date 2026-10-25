// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md: a `mutating` closure is many-call and is held in a
// `var` (calls are exclusive uses). Every call's assignments write back to the
// enclosing variables, so state accumulates across repeated calls — the
// captures are &mutating views, not snapshots.
module Test

@main
func main() -> lang.i64 {
    var total = 0;
    var count = 0;
    var bump: mutating (lang.i64) -> () = {
        total = lang.i64_add(total, it);
        count = lang.i64_add(count, 1);
    };
    bump(5);
    bump(7);
    bump(1);
    if lang.i64_eq(total, 13) { } else { return 1; }
    if lang.i64_eq(count, 3) { } else { return 2; }
    0
}
