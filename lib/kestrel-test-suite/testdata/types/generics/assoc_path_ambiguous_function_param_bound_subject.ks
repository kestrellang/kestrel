// test: diagnostics
// stdlib: true

// G29: the `Bound` counterpart of `assoc_path_ambiguous_function_param_equality.ks`.
// `T.Out: Show` names `Out`, which both `HasOutA` and `HasOutB` declare at the
// same level. Name resolution used to pick one by name and keep the bound;
// the subject is now re-decided by the same nearest-level rule and reported
// as E479 at the clause.

module Test

protocol HasOutA { type Out; func outA() -> Out }
protocol HasOutB { type Out; func outB() -> Out }
protocol Show { func show() -> Int64 }

func g[T](t: T) -> Int64 where T: HasOutA, T: HasOutB, T.Out: Show { // ERROR: associated type 'Out' in where clause is ambiguous: 'T' is bound by HasOutA and HasOutB, which each declare 'Out'
    0
}
