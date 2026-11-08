// test: execution
// stdlib: true
// expect-exit: 0

// `a?.0` chains a tuple index.

module Test

@main
func main() -> lang.i64 {
    let pair: (Int64, Int64)? = .Some((3, 4));
    let none: (Int64, Int64)? = .None;
    if pair?.0 != .Some(3) { return 1 }
    if pair?.1 != .Some(4) { return 2 }
    if none?.0 != .None { return 3 }
    0
}
