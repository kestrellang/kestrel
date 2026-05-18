// test: execution
// stdlib: true

// Opaque type nested in Optional: `-> some Shape?`

module Test

protocol Shape {
    func area() -> std.numeric.Int64
}

struct Circle {
    let r: std.numeric.Int64
    public init(r r: std.numeric.Int64) { self.r = r }
}

extend Circle: Shape {
    public func area() -> std.numeric.Int64 { self.r * self.r }
}

func maybeShape(flag: std.core.Bool) -> std.result.Optional[some Shape] {
    if flag { return Circle(r: 3) }
    null
}

func main() -> lang.i64 {
    match maybeShape(true) {
        some s => {
            if s.area() != 9 { return 1 }
        },
        null => { return 2 }
    }
    match maybeShape(false) {
        some _ => { return 3 },
        null => {}
    }
    0
}
