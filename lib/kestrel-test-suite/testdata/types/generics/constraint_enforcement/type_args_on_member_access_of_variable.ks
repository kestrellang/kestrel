// test: diagnostics
// stdlib: true

module Test

struct Foo: Cloneable {
    var items: Array[lang.i64]
    func bar() -> lang.i64 {
        return self.items[lang.i64] // ERROR: type
    }
    func clone() -> Foo {
        Foo(items: self.items.clone())
    }
}
