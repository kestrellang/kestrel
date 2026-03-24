// test: diagnostics
// stdlib: false

module Main

struct Point { let x: lang.i64 }
struct A { func methodA() -> lang.i64 { 42 } }
struct B { let value: lang.i64 }
struct Counter {
    let value: lang.i64
    func getValue() -> lang.i64 { 42 }
}

func test(p: Point, b: B) -> lang.i64 {
    p.nonExistent() // ERROR:
    b.methodA()
    Counter.getValue()
}
