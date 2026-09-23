// test: diagnostics
// stdlib: true

// G29 nearest-level rule. `T` is bound by `P` and `Q`. `P` declares `Out`
// itself; `Q` declares no `Out` but inherits one from its parent `R`. A
// declaration on a directly bound protocol hides an inherited one, so
// `T.Out` is `P.Out`: no E479, and both the equality and the projection bound
// are kept and used by the bodies.
//
// No concrete conformer: a type conforming to both `P` and `R` has two
// requirements named `Out` and is rejected on its own (E445).
//
// Source-order twin: `assoc_path_nearest_level_hides_inherited_order.ks`.

module Test

protocol R { type Out; func outR() -> Out }
protocol Q: R { func q() -> Int64 }
protocol P { type Out; func outP() -> Out }
protocol Show { func show() -> Int64 }

func viaEquality[T](t: T) -> Int64 where T: P, T: Q, T.Out = Int64 {
    t.outP()
}

func viaBound[T](t: T) -> Int64 where T: P, T: Q, T.Out: Show {
    t.outP().show()
}
