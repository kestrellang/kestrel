// test: diagnostics
// stdlib: false

module Main

struct Empty {
    func doNothing() -> () { }
}

struct Outer {
    let inner: Inner
    func getInner() -> Inner { self.inner }
}

struct Inner {
    let value: lang.i64
    func getValue() -> lang.i64 { self.value }
}

struct Builder {
    let value: lang.i64
    func withValue(v: lang.i64) -> Builder { self }
}

struct Point {
    let x: lang.i64
    func getX() -> lang.i64 { self.x }
    func printX() -> lang.i64 { self.x }
    func copyX() -> lang.i64 { self.x }
}
