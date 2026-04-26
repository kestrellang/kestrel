// test: diagnostics
// stdlib: false

// Regression: calling a struct init with an argument label that doesn't
// match the init's parameter declaration must produce a diagnostic, not
// silently succeed.
//
// Previously: `gen_struct_init` walked through the `matched.is_empty()` arm
// with only a ktrace, emitting no error and no arg constraints. MIR's
// `resolve_init_function` then fell back to the first init regardless of
// label match, so the call lowered against an unintended init with arg
// types as MirTy::Error — silently miscompiling primitive arguments.

module Test

struct Box {
    var v0: lang.i64

    // Single-name init param: per Kestrel rules this is positional only
    // (call sites must use `Box(30)`, not `Box(v0: 30)`). The labeled
    // call site below is a real label mismatch.
    public init(v0: lang.i64) { self.v0 = v0 }
}

func test() -> Box {
    Box(v0: 30) // ERROR: no matching overload
}
