// test: diagnostics
// stdlib: false
// executable: true

// A valid `@main` (free function returning `()`) produces no diagnostics even
// in an executable build — neither E615/E616 nor the missing-entry-point E618.

module Test

@main
func main() { }
