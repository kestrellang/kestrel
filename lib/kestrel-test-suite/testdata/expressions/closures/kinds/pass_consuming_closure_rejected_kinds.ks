// test: diagnostics
// stdlib: false

// Passing table, `consuming` row (docs/design/closures.md — "Passing: What
// Fits Where"): a `consuming` closure value is one-shot and uniquely owned, so
// it fits no multi-call slot (→ normal ✗, → `mutating` ✗) and cannot become a
// shared, duplicable `escaping` handle (→ `escaping` ✗).
module Test

func takesNormal(f: () -> lang.i64) -> lang.i64 { f() }

func takesMutating(mutating f: mutating () -> lang.i64) -> lang.i64 { f() }

func takesEscaping(f: escaping () -> lang.i64) -> lang.i64 { f() }

// consuming -> normal: one-shot values are not many-call.
func consumingToNormal(x: lang.i64) -> lang.i64 {
    let c: consuming () -> lang.i64 = { x };
    takesNormal(c) // ERROR(E624)
}

// consuming -> mutating: one-shot values are not many-call.
func consumingToMutating(x: lang.i64) -> lang.i64 {
    var c: consuming () -> lang.i64 = { x };
    takesMutating(c) // ERROR(E624)
}

// consuming -> escaping: a unique one-shot owner is not a shared handle.
func consumingToEscaping(x: lang.i64) -> lang.i64 {
    let c: consuming () -> lang.i64 = { x };
    takesEscaping(c) // ERROR(E624)
}
