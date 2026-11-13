// test: diagnostics

module Main
import std.streams.println
import std.result.Optional

func main() -> lang.i64 {
    let x: Int? = .Some(42);
     println(x ?? 0);
    0
}
