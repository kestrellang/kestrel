// test: diagnostics
// stdlib: false

module Test

struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    func describe() -> lang.str { return "a point"; }
}
