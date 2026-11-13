// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Same as closure_assigned_takes_target_type.ks, with a literal index: the
// subscript can't resolve until the literal `0` is typed, so the target is
// settled (literal index included) before the closure is checked against it.
@main
func main() -> lang.i64 {
    let s = "abcd";
    var list: [escaping (&String) -> Int64] = [{ it.bytes.count }];
    list(0) = { it.bytes.count + 3 };
    if list(0)(s) != 7 { return 1 }
    0
}
