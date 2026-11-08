// test: diagnostics
// stdlib: false

module Test

struct Point {}

@deprecated("geometry moved to 'Geo'")
extend Point {
    func area() -> lang.i64 { 0 }
}

func use(p: Point) -> lang.i64 {
    p.area() // WARN: 'area' is deprecated: geometry moved to 'Geo'
}
