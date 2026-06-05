// test: execution
// expect-stdout: 42\n

module Main
import std.io.stdio.println

struct Factory {
    static func create() -> std.numeric.Int64 = 42
}

@main
func main() -> lang.i64 {
    let _ = println(Factory.create());
    0
}
