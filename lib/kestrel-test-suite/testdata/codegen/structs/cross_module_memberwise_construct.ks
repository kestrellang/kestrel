// test: execution
// stdlib: true
// include: _cross_module_memberwise_user.ks
// expect-exit: 0

// Regression: `StructFromSiblingModule(labeled: ...)` memberwise-constructed
// from a module that is MIR-lowered BEFORE the struct's own module used to
// collapse field names to `_0`, `_1`, … because
// `body_lower::call_to_value` looked up the struct in `ctx.module.structs`
// only. Codegen then rejected the construction with "field '_0' not found on
// struct 'Sdl.Rectangle'" (the sdl_pong blocker).
//
// The include helper (UserSide.buildPoint) lowers first — at that point
// Test.Point is not yet in `module.structs`.

module Test

import std.text.(String)
import UserSide.(buildPoint)

public struct Point {
    public var x: lang.i64
    public var y: lang.i64
}

@main
func main() -> lang.i64 {
    let p = buildPoint(10, 20);
    // Field reads also walk `module.structs`, so these exercise both the
    // construct path and the field-access path for a cross-module struct.
    lang.i64_sub(lang.i64_add(p.x, p.y), 30)
}
