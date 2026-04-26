// test: diagnostics
// stdlib: false

module Test
protocol Describable { func describe() -> lang.str }
struct Point { var x: lang.i64; var y: lang.i64 }
extend Point: Describable { } // ERROR: does not implement method 'describe'
