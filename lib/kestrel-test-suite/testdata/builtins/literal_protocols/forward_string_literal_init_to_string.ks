// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A custom string-literal type forwards its literal to `String` by calling
// its `init(stringLiteral ptr:, length:)` directly. `length` is a positional
// param, so the call is `String(stringLiteral: ptr, length)`.

struct Name: ExpressibleByStringLiteral {
    var text: String
    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.text = String(stringLiteral: ptr, length)
    }
}

@main
func main() -> lang.i64 {
    let n: Name = "hello";
    if n.text == "hello" { 0 } else { 1 }
}
