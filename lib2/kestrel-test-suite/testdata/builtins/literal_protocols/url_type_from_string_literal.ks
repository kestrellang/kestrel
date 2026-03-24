// test: diagnostics
// stdlib: false

module Test
struct URL: Prelude.ExpressibleByStringLiteral {
    var path: lang.str

    init(stringLiteral value: lang.str) {
        self.path = value
    }
}
func fetch(url: URL) { }
func test() {
    let url: URL = "https://example.com";
    fetch(url)
}
