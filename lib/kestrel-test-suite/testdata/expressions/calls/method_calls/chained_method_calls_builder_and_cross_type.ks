// test: diagnostics
// stdlib: false

module Main

struct Builder {
    let value: lang.i64
    func step1() -> Builder { self }
    func step2() -> Builder { self }
    func build() -> lang.i64 { self.value }
}

struct Container {
    let inner: Inner
    func getInner() -> Inner { self.inner }
}

struct Inner {
    let value: lang.i64
    func getValue() -> lang.i64 { self.value }
}

func test(b: Builder, c: Container) -> lang.i64 {
    b.step1().step2().build()
}
