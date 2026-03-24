// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

struct Rectangle {
    let origin: Point
    let width: lang.i64
    let height: lang.i64
}
