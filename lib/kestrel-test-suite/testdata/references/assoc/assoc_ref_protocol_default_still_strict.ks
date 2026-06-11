// test: diagnostics
// stdlib: true

// Stage 2d: the formation carve covers assoc-type BINDINGS (member
// aliases of structs/enums/extensions). An assoc-type DECL's default in
// a protocol body stays Strict — defaults flow through projection
// machinery with no per-use position re-check.
module Test

protocol P {
    type Item = &Int64 // ERROR(E489)
}
