// test: diagnostics
// stdlib: true

// G29: source-order twin of `assoc_path_nearest_level_hides_inherited.ks`,
// with the bounds written `T: Q, T: P` instead of `T: P, T: Q`. The where
// clause resolves `T.Out` to `P.Out` either way (nearest level: `P` declares
// `Out`, `Q` only inherits `R.Out`), so this should compile clean.
//
// EXPECTED TO FAIL today, and newly so since G29's nearest-level rule: the
// body emitters still look an associated type up again **by name** through
// the bounds in source order (`find_assoc_type_in_bounds`, the
// `Constraint::Associated` half of G29), reach `R.Out` through `Q` first, and
// disagree with the clause. Before the rule, name resolution made the same
// first-match pick (`R.Out`) for the clause, so the two layers agreed on the
// wrong one and this compiled. Flips to passing when that constraint is keyed
// by entity.

module Test

protocol R { type Out; func outR() -> Out }
protocol Q: R { func q() -> Int64 }
protocol P { type Out; func outP() -> Out }
protocol Show { func show() -> Int64 }

func viaEquality[T](t: T) -> Int64 where T: Q, T: P, T.Out = Int64 {
    t.outP()
}

func viaBound[T](t: T) -> Int64 where T: Q, T: P, T.Out: Show {
    t.outP().show()
}
