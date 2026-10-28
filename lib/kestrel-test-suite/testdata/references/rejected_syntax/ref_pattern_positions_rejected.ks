// test: diagnostics
// stdlib: false

// `&` binder patterns parse everywhere a binding pattern is legal, but are
// rejected outside `match` arm binder position (E211). A destructuring
// `let` is rejected because the desugared scrutinee temp dies inside the
// wrapper block — the binding would dangle. (Match-arm support is the
// stage-1.5 item-2 place-mode lowering; until it lands the match position
// is pinned in ref_pattern_match_inert_rejected.ks.)
module Test

func f() {
    let (&x, y) = (1, 2); // ERROR(E211)
}
