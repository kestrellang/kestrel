// test: diagnostics
// stdlib: false
module Test

struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    private func internalSum() -> lang.i64 { return lang.i64_add(self.x, self.y); }
    func doubleSum() -> lang.i64 { return lang.i64_mul(self.internalSum(), 2); }
}
