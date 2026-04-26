// test: execution
// expect-stdout: 42\n

module Main
import std.io.stdio.println

protocol Valuable {
    func value() -> std.num.Int64
}

struct Thing: Valuable {
    let n: std.num.Int64

    func value() -> std.num.Int64 = self.n * 2
}

func main() -> std.num.Int64 {
    let t = Thing(n: 21);
    let _ = println(t.value());
    0
}
