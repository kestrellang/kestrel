// test: execution
// stdlib: true
// expect-stdout: [    42]\n[3.142]\n[000007]\n[  hi  ]\n[2.500 ]\n[ab        1.50]\n

// A nested hole in a format spec reads the width or precision at runtime:
// `\(n:>{w})`, `\(x:.{p}f)`. It names a local or a dotted path.
module Test

struct Table {
    var width: Int64
    func row(label: String, value: Float64) -> String { "[\(label:<{self.width}) \(value:>8.2f)]" }
}

@main
func main() -> Int64 {
    let w = 6;
    let p = 3;
    println("[\(42:>{w})]");
    println("[\(3.14159:.{p}f)]");
    println("[\(7:0{w})]");
    println("[\("hi":^{w})]");
    let x = 2.5;
    println("[\(x:{w}.{p})]");
    println(Table(width: 5).row("ab", 1.5));
    0
}
