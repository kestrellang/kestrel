// test: diagnostics
// stdlib: false
module Test

protocol Hashable { func hash() -> lang.i64 }
struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    func hash() -> lang.i64 { return lang.i64_add(self.x, self.y); }
}
extend Point: Hashable { }
