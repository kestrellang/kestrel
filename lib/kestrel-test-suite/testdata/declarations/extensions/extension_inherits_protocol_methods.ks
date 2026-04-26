// test: diagnostics
// stdlib: false

module Test

protocol Base { func base() -> lang.i64 }
protocol Child: Base { func child() -> lang.i64 }
struct Point { var x: lang.i64; var y: lang.i64 }
extend Point: Child {
    func base() -> lang.i64 { return self.x; }
    func child() -> lang.i64 { return self.y; }
}
