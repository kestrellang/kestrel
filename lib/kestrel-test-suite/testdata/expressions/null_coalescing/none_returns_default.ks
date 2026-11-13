// test: diagnostics

module Main
import std.streams.println
import std.result.Optional

func main() -> lang.i64 {
    let x: Int? = .None;
     println(x ?? 99);
    0
}
