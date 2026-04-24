// test: execution
// stdlib: false
// expect-exit: 42

// Regression: `apply_callee_param_modes` used to silently do nothing when
// the callee FunctionDef wasn't yet in `ctx.module.functions`. That
// happens for forward references within a module — `item::lower_member_functions`
// walks children one at a time and `function_lower::lower_function_sig`
// interleaves sig-add + body-lower, so calls to a later-defined sibling
// miss. Without the mode propagation, `mutating x` fell back to pass-by-
// value and callee mutations were lost.
//
// `main` (lowers first) calls `bump` (lowers second). If the ECS fallback
// applies the `mutating` → `MutRef` mode, the box is updated in place and
// main returns 42. Without the fallback, the box is passed by value, bump
// mutates a local copy, and main returns 5.

module Test

func main() -> lang.i64 {
    var n: lang.i64 = 5;
    bump(n);
    n
}

func bump(mutating x: lang.i64) {
    x = 42
}
