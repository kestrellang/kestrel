// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A custom string-literal type forwards its literal to `String` by calling
// `String(stringLiteral:length:)` directly. That init is public, but the
// explicit call reports "no matching overload for 'String'" (E113). Found
// while writing `std.files.Path`, which now copies the bytes another way.

struct Name: ExpressibleByStringLiteral {
    var text: String
    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.text = String(stringLiteral: ptr, length: length)
    }
}

@main
func main() -> lang.i64 {
    let n: Name = "hello";
    if n.text == "hello" { 0 } else { 1 }
}
