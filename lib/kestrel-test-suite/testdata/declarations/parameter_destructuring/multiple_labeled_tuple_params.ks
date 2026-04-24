// test: diagnostics
// stdlib: false

module Main

func distance(from (x1, y1): (lang.i64, lang.i64), to (x2, y2): (lang.i64, lang.i64)) -> lang.i64 {
    let dx = lang.i64_sub(x2, x1);
    let dy = lang.i64_sub(y2, y1);
    lang.i64_add(lang.i64_mul(dx, dx), lang.i64_mul(dy, dy))
}

func test() -> lang.i64 {
    distance(from: (0, 0), to: (3, 4))
}
