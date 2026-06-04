// test: execution
// stdlib: true
// expect-stdout: 4\n

module Main
import std.io.stdio.println

enum Foo {
    case A
    static let staticLet: std.numeric.Int64 = 4;
}

@main
func main() -> lang.i64 {
    let _ = println(Foo.staticLet);
    0
}
