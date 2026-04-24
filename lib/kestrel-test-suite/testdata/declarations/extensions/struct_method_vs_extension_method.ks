// test: diagnostics
// stdlib: false
module Test
struct Point {
    var x: lang.i64; var y: lang.i64
    func sum() -> lang.i64 { return lang.i64_add(self.x, self.y); }
}
extend Point {
    func sum() -> lang.i64 { return 0; } // ERROR: duplicate
}
