// test: diagnostics
// stdlib: true

// A non-statement-like expression statement needs a `;`. The parser
// synthesises a zero-width `;` to recover, and the emitter must surface it
// (F13) — previously the whole body compiled with zero diagnostics.

module Test

func side() -> lang.i64 { 1 }

func run() {
    side() // ERROR: expected `;`
    side();
}
