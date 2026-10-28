// test: execution
// stdlib: false
// backends: cranelift,llvm

// Named ref binding aliasing (ratified may-alias semantics): the binding
// names the PLACE — a later write through the var is visible through the
// binding (the borrow views the var's slot; reads load at use time).
// Bindings are BLOCK-LOCAL: all ref work happens straight-line, asserts
// come after the last use (an `if` arm/merge would be the binding E497).
module Test

@main
func main() -> lang.i64 {
    var x: lang.i64 = 1;
    let r = &x;
    let before = r;
    x = 5;
    let after = r;
    if lang.i64_eq(before, 1) { } else { return 1; }
    if lang.i64_eq(after, 5) { } else { return 2; }
    0
}
