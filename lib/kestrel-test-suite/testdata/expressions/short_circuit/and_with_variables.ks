// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a = true;
    let b = false;
    let _ = println(a and b);
    let _ = println(a and a);
    0
}
