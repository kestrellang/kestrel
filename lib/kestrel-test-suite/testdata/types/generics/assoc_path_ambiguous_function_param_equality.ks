// test: diagnostics
// stdlib: true

// G29: on a function's own type parameter, name resolution resolves `T.Out`
// by name — it used to pick `HasOutA.Out` silently and keep the clause.
// `HasOutB` declares an `Out` too, at the same level, so the equality cannot
// say which it pins: E479 at the clause, exactly as for the holder-bounds
// fallback (`assoc_equality_ambiguous_across_holder_bounds.ks`).
//
// E479 is the only error. The dropped clause is replaced by error-typed
// stand-ins for each candidate (`T.<HasOutA.Out>`, `T.<HasOutB.Out>`), so the
// body's `outA()` absorbs instead of reporting a follow-on mismatch; the body
// emitters key an equality by its associated-type entity (G29).
//
// Control: `assoc_path_single_function_param_bound_runs.ks`.

module Test

protocol HasOutA { type Out; func outA() -> Out }
protocol HasOutB { type Out; func outB() -> Out }

func g[T](t: T) -> Int64 where T: HasOutA, T: HasOutB, T.Out = Int64 { // ERROR: associated type 'Out' in where clause is ambiguous: 'T' is bound by HasOutA and HasOutB, which each declare 'Out'
    t.outA()
}
