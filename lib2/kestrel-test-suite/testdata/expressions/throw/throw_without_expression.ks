// test: diagnostics
// stdlib: false

module Test
struct Error {}
func failing() -> Error {
    throw // ERROR: expected
}
