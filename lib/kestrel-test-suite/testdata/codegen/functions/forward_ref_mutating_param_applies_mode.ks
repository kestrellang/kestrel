// test: mir
// stdlib: false
// mir-filter: Test.caller

// Regression: `apply_callee_param_modes` used to silently do nothing when
// the callee FunctionDef wasn't yet in `ctx.module.functions`. That
// happens for forward references within a module — `lower_member_functions`
// walks children one at a time and `lower_function_sig` interleaves
// sig-add + body-lower, so a call to a later-defined sibling misses the
// lookup. Without the mode propagation, `mutating` args silently fell
// back to pass-by-copy/ref.
//
// `caller` (lowers first) calls `reset` (lowers second). The call should
// emit `call Test.reset(mut %pt)`; without the ECS fallback in
// `apply_callee_param_modes` it would emit `call Test.reset(ref %pt)`.

module Test

struct Point { var x: lang.i64; var y: lang.i64 }

func caller() {
    var pt = Point(x: 1, y: 2);
    reset(pt)
}

func reset(mutating p: Point) {
    p.x = 0
}
