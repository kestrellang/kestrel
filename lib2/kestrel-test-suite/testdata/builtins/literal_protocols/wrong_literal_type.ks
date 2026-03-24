// test: diagnostics
// stdlib: false

module Test
struct Name: Prelude.ExpressibleByStringLiteral {
    var value: lang.str

    init(stringLiteral value: lang.str) {
        self.value = value
    }
}
func test() -> Name {
    42
}
