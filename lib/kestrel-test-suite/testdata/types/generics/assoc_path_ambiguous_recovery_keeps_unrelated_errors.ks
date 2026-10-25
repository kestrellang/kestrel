// test: diagnostics
// stdlib: true

// G29: an associated-type clause dropped as ambiguous (E479) is replaced by
// error-typed stand-ins, so the body's uses of `T.Out` do not cascade. The
// recovery must reach only those uses: every other error in the same body is
// still reported. Covers the equality form and the bound-subject form.
//
// Siblings: `assoc_equality_ambiguous_across_holder_bounds.ks`,
// `assoc_path_ambiguous_function_param_equality.ks`,
// `assoc_path_ambiguous_function_param_bound_subject.ks`.

module Test

protocol HasOutA { type Out; func outA() -> Out }
protocol HasOutB { type Out; func outB() -> Out }
protocol Show { func show() -> Int64 }

func viaEquality[T](t: T) -> Int64 where T: HasOutA, T: HasOutB, T.Out = Int64 { // ERROR: associated type 'Out' in where clause is ambiguous: 'T' is bound by HasOutA and HasOutB, which each declare 'Out'
    let pinned: Int64 = t.outA();
    let unrelated: Bool = pinned; // ERROR: expected Bool got Int64
    pinned
}

func viaBound[T](t: T) -> Int64 where T: HasOutA, T: HasOutB, T.Out: Show { // ERROR: associated type 'Out' in where clause is ambiguous: 'T' is bound by HasOutA and HasOutB, which each declare 'Out'
    let shown = t.outB().show();
    t.missing() // ERROR: no member 'missing' on type 'T'
}
