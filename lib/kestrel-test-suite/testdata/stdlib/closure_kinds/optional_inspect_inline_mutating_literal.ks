// test: execution
// stdlib: true
// expect-exit: 0

// Pins `Optional.inspect` as a `mutating`-closure API (docs/design/closures.md,
// "mutating: write-back"; one of the five eager side-effect APIs in
// closures-stdlib-audit.md). The inline literal writes back to a captured frame
// var, and `inspect` still returns `self` unchanged.
module Test

@main
func main() -> lang.i64 {
    // Some: the tap runs and its assignment reaches the enclosing var.
    var seen: Int64 = 0;
    let present: Optional[Int64] = .Some(42);
    let tapped = present.inspect({ (x) in seen = x });
    if seen != 42 { return 1 }
    if tapped.isSome() == false { return 2 }
    if tapped.unwrap() != 42 { return 3 }

    // None: the tap never runs, so the captured place keeps its old value.
    var untouched: Int64 = 7;
    let none: Optional[Int64] = .None;
    let passed = none.inspect({ (x) in untouched = 0 });
    if untouched != 7 { return 4 }
    if passed.isSome() { return 5 }

    0
}
