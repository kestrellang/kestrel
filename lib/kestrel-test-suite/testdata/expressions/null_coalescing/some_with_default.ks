// test: diagnostics

module Main
import std.streams.println

func main() -> lang.i64 {
    let a: Int? = .Some(1);
     println(a ?? 99);
    0
}
