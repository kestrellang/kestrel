// test: diagnostics
// stdlib: false

// An executable build must have exactly one `@main`.

module Test

@main
func main() { } // ERROR(E617)

@main
func other() { } // ERROR(E617)
