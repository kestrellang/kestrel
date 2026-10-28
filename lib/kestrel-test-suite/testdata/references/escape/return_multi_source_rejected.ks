// test: diagnostics
// stdlib: false

// E-REF-12: provenance is inferred single-source — a ref-returning
// signature with TWO reference-eligible roots (here both borrow params)
// is ambiguous and cleanly rejected at the declaration.
module Test

func pick(a: lang.i64, b: lang.i64, c: lang.i1) -> &lang.i64 { // ERROR(E493)
    if c { a } else { b }
}
