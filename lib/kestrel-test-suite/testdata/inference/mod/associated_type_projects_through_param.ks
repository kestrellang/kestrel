// test: diagnostics
// stdlib: false

// Regression: T.Item in a generic return type must project through the
// concrete receiver type, not leak as the abstract TypeAlias name.
// Before fix: `-> T.Item` on a Box (with `type Item = lang.i64`) surfaced
// as "Item" in diagnostics and downstream "Item !: NotEqual" errors fired.
// After fix: T.Item lowers to HirTy::AssocProjection and the solver
// projects it to lang.i64 via the Container conformance extension.

module Main

protocol Container {
    type Item
    func fetch() -> Item
}

struct Box {
    var value: lang.i64
}

extend Box: Container {
    type Item = lang.i64
    func fetch() -> lang.i64 { self.value }
}

func first[T](c: T) -> T.Item where T: Container {
    c.fetch()
}

func test() -> lang.i64 {
    let b = Box(value: 42);
    first(b)
}
