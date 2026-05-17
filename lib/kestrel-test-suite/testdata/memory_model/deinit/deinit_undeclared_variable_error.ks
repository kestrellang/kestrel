// test: diagnostics
// stdlib: false

module Test
func example() {
    deinit unknown; // ERROR: undeclared
}
