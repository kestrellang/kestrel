// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func main() -> lang.i64 {
    let p = Point(x: 3, y: 4);
    lang.i64_add(p.x, p.y)
}
