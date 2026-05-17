// test: execution
// stdlib: true
// expect-exit: 0

module Test

func greet(name: String) -> String {
    "Hello, \(name)"
}

func main() -> lang.i64 {
    // Nested: function returning interpolated string, used inside interpolation
    if "\(greet("Kestrel"))!" != "Hello, Kestrel!" { return 1 }

    // String literal inside interpolation expression
    if "say \("hi")" != "say hi" { return 2 }

    0
}
