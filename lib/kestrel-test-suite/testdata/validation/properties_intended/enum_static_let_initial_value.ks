// test: execution
// stdlib: true
// expect-stdout: 4\n

module Main
import std.io.stdio.println

enum Foo {
    case A
    static let staticLet: std.numeric.Int64 = 4;
}

func main() -> std.numeric.Int64 {
    let _ = println(Foo.staticLet);
    0
}
