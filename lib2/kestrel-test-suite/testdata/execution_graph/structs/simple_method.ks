// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
    
    func distanceSquared() -> lang.i64 {
        lang.i64_add(lang.i64_mul(self.x, self.x), lang.i64_mul(self.y, self.y))
    }
}
