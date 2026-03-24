// test: diagnostics
// stdlib: false

module Test

protocol Container {
    type Element
    func first() -> Element
}

struct Processor {
    var count: lang.i64

    init[C](container: C) where C: Container, C.Element = lang.i64 {
        self.count = container.first()
    }
}
