// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

struct Values {
    let a: lang.i64
    let b: lang.i64
    let c: lang.i64
}

func add(p: Point) -> lang.i64 {
    lang.i64_add(p.x, p.y)
}

func compute(v: Values) -> lang.i64 {
    lang.i64_add(lang.i64_mul(v.a, v.b), v.c)
}
