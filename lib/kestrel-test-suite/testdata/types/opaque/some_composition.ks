// test: diagnostics
// stdlib: false

// `some P and Q` composition — methods from both protocols
// should be visible. No errors expected.

module Test

protocol Drawable {
    func draw() -> lang.i64
}

protocol Printable {
    func describe() -> lang.i64
}

struct Widget {
    public init() {}
}
extend Widget: Drawable {
    public func draw() -> lang.i64 { 1 }
}
extend Widget: Printable {
    public func describe() -> lang.i64 { 2 }
}

func make() -> some Drawable and Printable {
    Widget()
}

func test() {
    let w = make();
    w.draw();
    w.describe()
}
