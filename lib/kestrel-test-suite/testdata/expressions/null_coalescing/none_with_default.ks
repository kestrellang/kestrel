// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a: Int? = null;
    let _ = println(a ?? 99);
    0
}
