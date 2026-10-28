// test: execution
// stdlib: true
// expect-exit: 0
//
// #189 (BUG-53): splitting a user enum case's payload across arms is exhaustive
// when the payload's own cases are jointly covered — here `.Has(.Some(b))` +
// `.Has(.None)` cover `Optional[Int64]`, and with `.Nothing` the whole `W3` is
// covered. This used to be falsely rejected as non-exhaustive (E305) because
// the payload field type `Optional[Int64]` was re-resolved by a hand-rolled
// resolver that returned `Error` for any generic instantiation, so `.Some`/
// `.None` couldn't be matched against it. Compiling (no E305) and running
// proves the payload type now resolves and the split is exhaustive.

module Test

enum W3 {
    case Has(Optional[Int64])
    case Nothing
}

func f(w: W3) -> Int64 {
    match w {
        .Has(.Some(b)) => b,
        .Has(.None) => 0,
        .Nothing => -1
    }
}

@main
func main() -> lang.i32 {
    if f(.Has(.Some(2))) != 2 { return 1 }
    if f(.Has(.None)) != 0 { return 2 }
    if f(.Nothing) != -1 { return 3 }
    0
}
