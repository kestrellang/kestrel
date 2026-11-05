// test: diagnostics

module Main
import std.streams.println

func main() -> lang.i64 {
    let a = true;
    let b = false;
     println(a or b);
     println(b or b);
    0
}
