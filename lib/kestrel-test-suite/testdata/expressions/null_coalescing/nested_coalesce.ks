// test: diagnostics

module Main
import std.streams.println

func main() -> lang.i64 {
    let a: Int? = .Some(5);
    let b: Int? = null;
     println((a ?? 10) + (b ?? 20));
    0
}
