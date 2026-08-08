// test: diagnostics
// stdlib: false

// Passing table, `normal` row (docs/design/closures.md — "Passing: What Fits
// Where"): a normal closure VALUE holds frame views, so it is neither owned
// (→ `consuming` ✗) nor detachable (→ `escaping` ✗). Each rejected cell lives
// in its own function so one E624 cannot mask the next.
module Test

func takesConsuming(consuming f: consuming () -> lang.i64) -> lang.i64 { f() }

func takesEscaping(f: escaping () -> lang.i64) -> lang.i64 { f() }

// normal -> consuming: a frame view is not an owned environment.
func normalToConsuming(x: lang.i64) -> lang.i64 {
    let n: () -> lang.i64 = { x };
    takesConsuming(n) // ERROR(E624)
}

// normal -> escaping: frame-bound values never flow into an escaping slot.
func normalToEscaping(x: lang.i64) -> lang.i64 {
    let n: () -> lang.i64 = { x };
    takesEscaping(n) // ERROR(E624)
}
