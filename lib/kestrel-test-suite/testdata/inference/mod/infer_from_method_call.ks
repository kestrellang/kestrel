// test: diagnostics
// stdlib: false

module Main

struct Foo {
    func bar() -> lang.i64 { 42 }
}

func test() -> lang.i64 {
    let f = Foo();
    let x = f.bar();
    x
}
