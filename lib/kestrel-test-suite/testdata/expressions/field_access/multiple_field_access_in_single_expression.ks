// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func sum(p: Point) -> lang.i64 {
    lang.i64_add(p.x, p.y)
}
