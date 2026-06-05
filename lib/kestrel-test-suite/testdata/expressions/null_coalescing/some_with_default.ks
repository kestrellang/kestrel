// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a: Int? = .Some(1);
     println(a ?? 99);
    0
}
