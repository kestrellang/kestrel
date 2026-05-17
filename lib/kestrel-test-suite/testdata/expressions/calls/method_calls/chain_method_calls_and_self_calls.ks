// test: diagnostics
// stdlib: false

module Main

struct Builder {
    let value: lang.i64
    func build() -> lang.i64 { 42 }
}

struct Factory {
    let builder: Builder
    func getBuilder() -> Builder { self.builder }
    func buildResult() -> lang.i64 { self.getBuilder().build() }
}

struct Calculator {
    let value: lang.i64
    func getValue() -> lang.i64 { 42 }
    func getDoubleValue() -> lang.i64 { self.getValue() }
}
