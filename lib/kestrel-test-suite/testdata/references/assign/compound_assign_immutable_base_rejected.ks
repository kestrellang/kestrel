// test: diagnostics
// stdlib: true

// Stage 1.5: an accessor-backed member is a place PROJECTION — its
// mutability is the base's. RMW through a `let` base is rejected by the
// access-mode analyzer (the member itself has a write provider, so the
// old E202 "not assignable" no longer applies).
module Test

@main
func main() {
    let arr = [1, 2, 3];
    arr(0) += 10; // ERROR(E203)
}
