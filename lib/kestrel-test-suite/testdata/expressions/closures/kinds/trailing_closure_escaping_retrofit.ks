// test: execution
// stdlib: true
// expect-exit: 0

// Plan D3 (docs/plans/closure-kinds/closure-kinds-plan.md), the literal
// retrofit gate: closure literals are always built at `Normal` and the
// expected type retrofits the kind IN PLACE (`set_function_kind`), gated on
// `closure_literal_exprs`. That gate must UNWRAP `HirExpr::Sugar` / `Block`
// wrappers to find the literal id — trailing-closure call sites wrap the
// literal, so without the unwrapping the retrofit silently misses and the site
// fails with a spurious E624 (kind mismatch) or E603 (assign-to-capture).
//
// The observable proof that the retrofit landed is SNAPSHOT semantics: an
// owning `escaping` capture copies `base` at creation, so the later write is
// not visible through the stored closure. A missed retrofit would either fail
// to compile or leave a normal-kind frame VIEW that reads 100 (and could not
// have been stored in `Holder` at all).
module Test

import std.numeric.(Int64)

struct Holder {
    let action: escaping () -> Int64
}

func store(consuming action: escaping () -> Int64) -> Holder {
    Holder(action: action)
}

@main
func main() -> lang.i64 {
    var base = 10;
    let h = store { base + 1 };          // TRAILING closure — brace on the same line
    base = 99;                           // snapshot taken at creation, not read now

    if (h.action)() != 11 { return 1 }
    if (h.action)() != 11 { return 2 }   // escaping is many-call, and stable
    if base != 99 { return 3 }           // the owning capture left `base` untouched

    0
}
