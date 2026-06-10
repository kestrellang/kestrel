// test: diagnostics
// stdlib: false

// INERT-STAGE PIN (A1): `&` binders in match arms are rejected until the
// place-mode match lowering lands (stage 1.5 item 2 / A5) — this file is
// then REPLACED by execution tests in references/patterns/.
module Test

enum Box {
    case Full(lang.i64)
    case Empty
}

func f(b: Box) -> lang.i64 {
    match b {
        .Full(&v) => v, // ERROR(E211)
        .Empty => 0
    }
}
