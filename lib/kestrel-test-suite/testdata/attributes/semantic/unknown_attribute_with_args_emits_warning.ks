// test: diagnostics
// stdlib: false

module Test
@customThing(key: "value") // WARN: unknown attribute
func bar() {}
