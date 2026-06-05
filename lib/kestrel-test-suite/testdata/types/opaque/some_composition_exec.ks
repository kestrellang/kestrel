// test: execution
// stdlib: true

// `some P and Q` composition — methods from both protocols work at runtime.

module Test

protocol Drawable {
    func draw() -> std.numeric.Int64
}

protocol Printable {
    func describe() -> std.numeric.Int64
}

struct Widget {
    public init() {}
}

extend Widget: Drawable {
    public func draw() -> std.numeric.Int64 { 10 }
}

extend Widget: Printable {
    public func describe() -> std.numeric.Int64 { 20 }
}

func make() -> some Drawable and Printable {
    Widget()
}

@main
func main() -> lang.i64 {
    let w = make();
    if w.draw() != 10 { return 1 }
    if w.describe() != 20 { return 2 }
    0
}
