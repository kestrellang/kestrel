// test: diagnostics
// stdlib: false

module Test

protocol Hashable { func hash() -> lang.i64 }
protocol Describable { func describe() -> lang.str }
struct Point { var x: lang.i64; var y: lang.i64 }
extend Point: Hashable, Describable {
    func hash() -> lang.i64 { return lang.i64_add(self.x, self.y); }
    func describe() -> lang.str { return "point"; }
}
