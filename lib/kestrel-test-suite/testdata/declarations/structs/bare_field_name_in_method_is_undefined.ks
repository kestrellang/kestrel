// test: diagnostics
// stdlib: true

// Kestrel has no implicit `self` for member access: fields are read as
// `self.count` (docs/language/structs.md), and the same bare name inside an
// `extend S` method is already "undefined name 'count'". Inside the struct
// body, though, `ScopeFor(struct)` lists instance fields as lexical
// declarations, so `count` resolves to the field entity, passes every
// front-end and analysis stage, and only fails in codegen with "unsupported:
// global entity not found in statics". Found in the 2026-10 architecture
// review at 9767d2dc.
// EXPECTED TO FAIL until instance members are never lexical bindings.
module Test

struct S {
    var count: Int64;

    func f() -> Int64 { count } // ERROR: undefined name 'count'
}
