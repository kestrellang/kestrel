// test: execution
// stdlib: true
// expect-exit: 0

// Pins `Result.inspect` as a `mutating`-closure API (docs/design/closures.md,
// "mutating: write-back"; one of the five eager side-effect APIs in
// closures-stdlib-audit.md). The inline literal writes back to a captured frame
// var on the Ok branch only, and `inspect` returns `self` unchanged.
module Test

@main
func main() -> lang.i64 {
    // Ok: the tap runs and its assignment reaches the enclosing var.
    var seen: Int64 = 0;
    let ok: Result[Int64, Int64] = .Ok(42);
    let tapped = ok.inspect({ (x) in seen = x });
    if seen != 42 { return 1 }
    if tapped.isOk() == false { return 2 }
    if tapped.unwrap() != 42 { return 3 }

    // Err: the Ok-branch tap never runs; the captured place is untouched.
    var untouched: Int64 = 9;
    let err: Result[Int64, Int64] = .Err(99);
    let passed = err.inspect({ (x) in untouched = 0 });
    if untouched != 9 { return 4 }
    if passed.isErr() == false { return 5 }
    if passed.unwrapErr() != 99 { return 6 }

    0
}
