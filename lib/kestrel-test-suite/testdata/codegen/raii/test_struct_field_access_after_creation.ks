// test: diagnostics
// stdlib: false

module Test

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func main() -> lang.i64 {
    let p = Point(x: 10, y: 20);
    lang.i64_add(p.x, p.y)
}
