// test: diagnostics
// stdlib: false

module Main

struct Widget {
    let id: lang.i64
    public func getId() -> lang.i64 { self.id }
    private func internalId() -> lang.i64 { self.id }
    func getInternalId() -> lang.i64 { self.internalId() }
}

func test(w: Widget) -> lang.i64 {
    w.getId()
}
