// test: diagnostics
// stdlib: false

// `&` binders are allowed in every pattern position, but a destructuring
// `let` of a TEMPORARY is rejected (E499): the desugared scrutinee temp
// dies inside the wrapper block, so the binding would dangle. A `let`
// destructure of a place is matched in place and is fine (see
// patterns/ref_binders_in_every_position.ks).
module Test

func f() {
    let (&x, y) = (1, 2); // ERROR(E499)
}
