// test: diagnostics
// stdlib: false
module Test
struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    func foo() -> lang.i64 { return 1; }
}
extend Point {
    func foo() -> lang.i64 { return 2; } // ERROR: duplicate
}
