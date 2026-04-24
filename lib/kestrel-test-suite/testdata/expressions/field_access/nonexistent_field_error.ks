// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func getZ(p: Point) -> lang.i64 {
    p.z // ERROR: no member 'z' on type 'Point'
}
