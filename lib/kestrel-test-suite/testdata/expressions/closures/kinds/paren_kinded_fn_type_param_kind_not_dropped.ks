// test: diagnostics
// stdlib: false

// Plan D2 (docs/plans/closure-kinds/closure-kinds-plan.md), the third grammar
// reading: `(mutating (T) -> R) -> U` must read as a function type whose single
// PARAM is a mutating-KIND closure — NOT as a MutBorrow-convention parameter of
// a NORMAL function type, which is what the greedy `or_not()` on `Mutating` in
// the `elem` parser (ty/mod.rs:180-188) produces today.
//
// The two readings are observationally different at exactly this site.
// `normalTaker`'s parameter is a normal-kind closure, so it matches only the
// WRONG reading (where the kind was dropped and the convention absorbed the
// keyword). Under the correct reading the parameter kinds differ — normal vs
// mutating — and kind equality in `unify` rejects the assignment.
//
// Positive counterpart (readings 1 and 2, exercised by calls):
// expressions/closures/kinds/paren_kinded_fn_type_readings.ks
module Test

func normalTaker(f: (lang.i64) -> ()) {}

func test() {
    let g: (mutating (lang.i64) -> ()) -> () = normalTaker; // ERROR(E624)
}
