// test: execution
// expect-stdout: 42\n

module Main
import std.io.stdio.println

protocol Valuable {
    func value() -> std.numeric.Int64
}

struct Thing: Valuable {
    let n: std.numeric.Int64

    func value() -> std.numeric.Int64 = self.n * 2
}

@main
func main() -> lang.i64 {
    let t = Thing(n: 21);
    let _ = println(t.value());
    0
}
