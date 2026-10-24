// test: diagnostics
// stdlib: false

module Test
@myCustomAttr(key: "value", 42) // ERROR: unknown attribute
func bar() {}
