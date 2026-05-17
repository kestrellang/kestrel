// test: diagnostics
// stdlib: false

module Test
protocol Printable {}

@dummy
struct Point: Printable {
    var x: lang.i64
}
