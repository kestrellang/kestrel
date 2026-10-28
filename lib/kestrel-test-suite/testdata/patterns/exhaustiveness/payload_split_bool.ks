// test: execution
// stdlib: true
// expect-exit: 0
//
// #189 (BUG-53): the Bool-typed-payload variant. Splitting a `Bool` payload
// across `.Has(true)`/`.Has(false)` jointly covers it, so with `.Nothing` the
// match is exhaustive. Like the generic case, `Bool` could not be resolved by
// the old sibling-scope payload-type resolver (a builtin, not a sibling), so
// the split was falsely flagged non-exhaustive.

module Test

enum W {
    case Has(Bool)
    case Nothing
}

func f(w: W) -> Int64 {
    match w {
        .Has(true) => 1,
        .Has(false) => 2,
        .Nothing => 3
    }
}

@main
func main() -> lang.i32 {
    if f(.Has(true)) != 1 { return 1 }
    if f(.Has(false)) != 2 { return 2 }
    if f(.Nothing) != 3 { return 3 }
    0
}
