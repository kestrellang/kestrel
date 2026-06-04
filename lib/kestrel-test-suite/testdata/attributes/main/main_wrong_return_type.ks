// test: diagnostics
// stdlib: false

// `@main` may return `()` or a primitive integer (lang.i8/i16/i32/i64).
// A float return type is rejected.

module Test

@main
func main() -> lang.f64 { 0.0 } // ERROR(E616)
