// test: diagnostics
// stdlib: false

module Test

struct Foo {
    var items: Array[lang.i64]
    func bar() -> lang.i64 {
        return self.items[lang.i64] // ERROR: type
    }
}
