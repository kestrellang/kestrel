// test: diagnostics
// stdlib: true

// Regression (E412): two extensions conforming a type to the SAME
// parameterized protocol with DIFFERENT type arguments — each providing the
// protocol's required method — must NOT be flagged as duplicate methods. The
// E412 check compared only method name + param labels (never each extension's
// conformed protocol/instantiation), so `Subtractable[Vec2]` and
// `Subtractable[Int64]` both defining `subtract` tripped a false
// "duplicate method 'subtract'" error. (Mirrors datetime's
// `Instant: Subtractable[Duration]` + `Subtractable[Instant]`.)
module Test

struct Vec2 { var x: std.numeric.Int64; var y: std.numeric.Int64 }

extend Vec2: std.core.Subtractable[Vec2] {
    type Output = Vec2
    consuming func subtract(consuming other: Vec2) -> Vec2 {
        Vec2(x: self.x - other.x, y: self.y - other.y)
    }
}

extend Vec2: std.core.Subtractable[std.numeric.Int64] {
    type Output = Vec2
    consuming func subtract(consuming other: std.numeric.Int64) -> Vec2 {
        Vec2(x: self.x - other, y: self.y - other)
    }
}
