// test: execution
// expect-stdout: 0\n

module Main
import std.io.stdio.println

struct Counter {
    var count: std.numeric.Int64

    mutating func reset() -> () = self.count = 0
}

@main
func main() -> lang.i64 {
    var c = Counter(count: 10);
    c.reset();
    let _ = println(c.count);
    0
}
