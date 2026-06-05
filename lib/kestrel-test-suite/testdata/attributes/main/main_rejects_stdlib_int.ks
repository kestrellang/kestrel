// test: diagnostics

// The entry point speaks the raw C-ABI boundary: the stdlib `Int64` struct is
// rejected — `@main` must return `()` or a `lang` primitive integer.

module Test

@main
func main() -> std.numeric.Int64 { 0 } // ERROR(E616)
