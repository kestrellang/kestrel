// test: diagnostics
// stdlib: true

// Expression statement followed by a `let`. The `let` keyword ends the
// expression, so the parser synthesises the `;` — which must be reported.

module Test

func side() -> lang.i64 { 1 }

func run() -> lang.i64 {
    side() // ERROR: expected `;`
    let x = 2;
    x
}
