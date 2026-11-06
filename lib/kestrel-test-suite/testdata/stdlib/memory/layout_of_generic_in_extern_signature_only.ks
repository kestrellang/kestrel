// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression: a generic instantiation mentioned ONLY in a bodyless `@extern`
// signature got no `MonoStruct`. Mono's layout seeding walked function
// bodies, and an extern has none, so `Pointer[Tm]` below was never seeded —
// while `verify_mono` checks signatures too, and rejected the build with
// "has no MonoStruct/MonoEnum — layout would silently fall back to a
// pointer-sized scalar". Found via `datetime`'s (unused) `gmtime_r`, which
// broke every program importing `datetime`.
//
// Seeding and the verifier now share one site list (`mono::ty_sites`), so any
// type the verifier can flag is seeded.

module Test

import std.ffi.(FFISafe)
import std.memory.(Pointer)

struct Tm: FFISafe {
    var sec: Int32
    var min: Int32
}

// Declared, never called: `Pointer[Tm]` appears nowhere else in the program.
@extern(.C)
func gmtime_r(timep: Pointer[Int64], result: Pointer[Tm]) -> Pointer[Tm]

@main
func main() -> lang.i64 {
    0
}
