// test: execution
// stdlib: true
// expect-exit: 0

// A user top-level module named after a builtin type breaks the whole stdlib:
// `module Int64` produces ~219 errors, all anchored inside lang/std (e.g.
// "expected Bool, found BooleanLiteralType"); `module Bool` produces ~67.
// `ResolveBuiltin` (name-res resolve_builtin.rs) tries a name lookup in the
// root scope BEFORE the `@builtin` index, and the root scope lists user
// top-level modules ahead of the std imports, so the `Default*LiteralType`
// builtins resolve to the user's module. Found in the 2026-10 architecture
// review at 9767d2dc.
// EXPECTED TO FAIL until builtins resolve through the `@builtin` index only.

module Int64

@main
func main() -> lang.i64 {
    if 1 < 2 { 0 } else { 1 }
}
