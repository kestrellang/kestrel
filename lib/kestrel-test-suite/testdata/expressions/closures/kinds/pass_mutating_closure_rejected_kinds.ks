// test: diagnostics
// stdlib: false

// Passing table, `mutating` row (docs/design/closures.md — "Passing: What Fits
// Where"): a `mutating` closure value fits only a `mutating` slot. Its calls
// are exclusive (→ normal ✗) and its environment is a frame view, so it is
// neither owned (→ `consuming` ✗) nor detachable (→ `escaping` ✗).
module Test

func takesNormal(f: () -> lang.i64) -> lang.i64 { f() }

func takesConsuming(consuming f: consuming () -> lang.i64) -> lang.i64 { f() }

func takesEscaping(f: escaping () -> lang.i64) -> lang.i64 { f() }

// mutating -> normal: exclusive-call values do not weaken to shared calls.
func mutatingToNormal() -> lang.i64 {
    var total = 0;
    var m: mutating () -> lang.i64 = { total = lang.i64_add(total, 1); total };
    takesNormal(m) // ERROR(E624)
}

// mutating -> consuming: a frame view is not an owned environment.
func mutatingToConsuming() -> lang.i64 {
    var total = 0;
    var m: mutating () -> lang.i64 = { total = lang.i64_add(total, 1); total };
    takesConsuming(m) // ERROR(E624)
}

// mutating -> escaping: frame-bound.
func mutatingToEscaping() -> lang.i64 {
    var total = 0;
    var m: mutating () -> lang.i64 = { total = lang.i64_add(total, 1); total };
    takesEscaping(m) // ERROR(E624)
}
