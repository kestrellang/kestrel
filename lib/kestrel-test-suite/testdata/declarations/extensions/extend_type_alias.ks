// test: diagnostics
// stdlib: false
module Test

struct Point { var x: lang.i64; var y: lang.i64 }
type MyPoint = Point;
extend MyPoint { func foo() { } } // ERROR: cannot extend
