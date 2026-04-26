// test: diagnostics
// stdlib: false

module Main

struct Counter {
    var value: lang.i64
    mutating func getValue() -> lang.i64 { self.value }
    mutating func increment() -> () { () }
}

struct Container {
    let item: lang.i64
    consuming func getItem() -> lang.i64 { self.item }
    consuming func take() -> lang.i64 { 42 }
}

func test(c: Counter, k: Container) -> lang.i64 {
    k.take()
}
