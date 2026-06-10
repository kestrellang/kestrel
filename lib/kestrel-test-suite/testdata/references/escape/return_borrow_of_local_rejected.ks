// test: diagnostics
// stdlib: false

// E-REF-10, the silent-UAF class: a returned reference whose provenance
// root is a function LOCAL must be rejected — the local dies at return.
// Only parameter-rooted or Pointer-derived references can be returned.
module Test

func bad() -> &lang.i64 {
    let x = 42;
    x // ERROR(E494)
}
