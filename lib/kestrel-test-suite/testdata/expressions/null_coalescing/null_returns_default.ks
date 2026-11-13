// test: diagnostics

module Main
import std.streams.println

func main() -> lang.i64 {
    let x: Int? = null;
     println(x ?? 123);
    0
}
