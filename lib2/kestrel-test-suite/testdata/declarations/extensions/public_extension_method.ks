// test: diagnostics
// stdlib: false
module Test

struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    public func sum() -> lang.i64 { return lang.i64_add(self.x, self.y); }
}
