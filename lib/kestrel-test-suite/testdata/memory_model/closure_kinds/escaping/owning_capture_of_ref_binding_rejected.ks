// test: diagnostics
// stdlib: false

// Plan D4 (docs/plans/closure-kinds/closure-kinds-plan.md), owning-tier
// capture rejections: an owning environment rejects "any capture whose type
// has `escape_carry(ty).any_ref`". A reference is Copyable, so it sails past
// the copy-class filter and would be bit-copied into an environment that may
// outlive the borrowed place — producing a returnable closure that holds a
// dangling reference (the rev-1 hole the review found).
//
// The VIEW kinds are the opposite case: they keep capturing ref bindings once
// E212 retires (plan D8 / lockstep 6, Phase F) — a view env is frame-bound, so
// the reference cannot outlive its referent. Only the OWNING kinds reject it.
//
// `&T` PARAMETERS are not a legal spelling today (E480, see
// references/rejected_syntax/param_ref_rejected.ks), so the reference here
// comes from a named ref binding — references/bindings/binding_aliases_var.ks.
module Test

func store(f: escaping () -> lang.i64) {}

func test() {
    var x: lang.i64 = 1;
    let r = &x;                                       // named ref binding
    store({ () in lang.i64_add(r, 1) }); // ERROR(E624)
}
