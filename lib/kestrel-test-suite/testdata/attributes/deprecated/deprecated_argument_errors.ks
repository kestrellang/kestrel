// test: diagnostics
// stdlib: false

module Test

@deprecated(renamed: "parse") // ERROR: '@deprecated' has no argument 'renamed'
func a() {}

@deprecated(since: 18) // ERROR: expected a string literal
func b() {}

@deprecated("one", "two") // ERROR: '@deprecated' takes at most one argument
func c() {}
