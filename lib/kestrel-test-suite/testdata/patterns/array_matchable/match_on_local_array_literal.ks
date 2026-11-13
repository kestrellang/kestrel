// test: execution
// stdlib: true
// expect-exit: 0
//
// An array pattern over a LOCAL array literal. The scrutinee is an @owned
// copy, and the owned path used to read `[a, ..]`'s `Index(0)` as the Array
// struct's first field (its CowBox) instead of a runtime `matchGet(0)`:
// an OSSA verify ICE as a function's tail `match`, and exit 15 as `main`'s.

module Test

func headLocal() -> Int64 {
    let xs: [Int64] = [5, 6, 7];
    match xs { [a, ..] => a, _ => 9 }
}

@main
func main() -> Int64 {
    if headLocal() != 5 { return 1 }
    let xs: [Int64] = [1, 2, 3];
    match xs { [a, ..] => a - 1, _ => 9 }
}
