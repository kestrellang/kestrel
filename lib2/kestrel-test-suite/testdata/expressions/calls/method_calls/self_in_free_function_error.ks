// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
}

func freeFunc() -> lang.i64 {
    self.x // ERROR: cannot use 'self' in free function
}
