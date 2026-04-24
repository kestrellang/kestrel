// test: execution
// expect-stdout: 0\n

module Main
import std.io.stdio.println

struct Counter {
    var count: std.num.Int64

    mutating func reset() -> () = self.count = 0
}

func main() -> std.num.Int64 {
    var c = Counter(count: 10);
    c.reset();
    let _ = println(c.count);
    0
}
