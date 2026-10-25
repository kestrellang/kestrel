// test: execution
// stdlib: true
// expect-exit: 0

// Pins `Result.inspectErr` as a `mutating`-closure API (docs/design/closures.md,
// "mutating: write-back"; the last of the five eager side-effect APIs in
// closures-stdlib-audit.md). The inline literal writes back to a captured frame
// var on the Err branch only, and `inspectErr` returns `self` unchanged.
module Test

@main
func main() -> lang.i64 {
    // Err: the tap runs and its assignment reaches the enclosing var.
    var seen: Int64 = 0;
    let err: Result[Int64, Int64] = .Err(99);
    let tapped = err.inspectErr({ (e) in seen = e });
    if seen != 99 { return 1 }
    if tapped.isErr() == false { return 2 }
    if tapped.unwrapErr() != 99 { return 3 }

    // Ok: the Err-branch tap never runs; the captured place is untouched.
    var untouched: Int64 = 5;
    let ok: Result[Int64, Int64] = .Ok(42);
    let passed = ok.inspectErr({ (e) in untouched = 0 });
    if untouched != 5 { return 4 }
    if passed.isOk() == false { return 5 }
    if passed.unwrap() != 42 { return 6 }

    0
}
