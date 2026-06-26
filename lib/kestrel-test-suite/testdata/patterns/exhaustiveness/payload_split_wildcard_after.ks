// test: execution
// stdlib: true
// expect-exit: 0
//
// #190 (BUG-54): a wildcard arm following a payload split used to panic
// mir-lower (`enum case Entity(4294967295) has no Name`) — the nested
// `.Some`/`.None` resolved to a u32::MAX sentinel entity (the payload type
// `Optional[Int64]` resolved to `Error`), and the decision-tree builder then
// emitted that sentinel as a real switch case. Sibling of #189 (same root
// cause); together they were a trap, since #189's E305 fix-it suggestion is
// exactly the wildcard arm that panicked here.

module Test

enum W3 {
    case Has(Optional[Int64])
    case Nothing
}

func f(w: W3) -> Int64 {
    match w {
        .Has(.Some(b)) => b,
        .Has(.None) => 0,
        .Nothing => -1,
        _ => -2
    }
}

@main
func main() -> lang.i32 {
    if f(.Has(.Some(2))) != 2 { return 1 }
    if f(.Has(.None)) != 0 { return 2 }
    if f(.Nothing) != -1 { return 3 }
    0
}
