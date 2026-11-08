// test: diagnostics
// stdlib: false

module Test
@customThing(key: "value") // ERROR: unknown attribute
func bar() {}
