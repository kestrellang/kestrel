// test: diagnostics
// stdlib: false

// `@main` must be a free (module-level) function. On a method it's an error.

module Test

struct S {
    @main
    func run() { } // ERROR(E615)
}
