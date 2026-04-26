// test: diagnostics
// stdlib: false

module Test
@myCustomAttr(key: "value", 42) // WARN: unknown attribute
func bar() {}
