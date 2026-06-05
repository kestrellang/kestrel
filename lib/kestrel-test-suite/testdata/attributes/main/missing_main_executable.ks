// test: diagnostics
// stdlib: false
// executable: true

// An executable build with no `@main` is an error (E618). The requirement is
// gated on executable builds — `// executable: true` opts this diagnostics test
// in (a normal diagnostics/library/check run would not flag it). The error is a
// whole-program one; it's anchored to the last declaration in the build.

module Test

func notMain() { } // ERROR(E618)
