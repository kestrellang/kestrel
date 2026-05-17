// test: diagnostics
// stdlib: false
// include: _prelude_literal_protocols.ks

module Test
struct Name: Prelude.ExpressibleByStringLiteral {
    var value: lang.str

    init(stringLiteral value: lang.str) {
        self.value = value
    }
}
func test() -> Name {
    "Alice"
}
