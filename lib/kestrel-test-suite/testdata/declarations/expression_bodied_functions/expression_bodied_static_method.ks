// test: execution
// expect-stdout: 42\n

module Main
import std.io.stdio.println

struct Factory {
    static func create() -> std.num.Int64 = 42
}

func main() -> std.num.Int64 {
    let _ = println(Factory.create());
    0
}
