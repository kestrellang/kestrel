// test: diagnostics

module Main
import std.io.stdio.println
import std.result.Optional

func main() -> lang.i64 {
    let x: Int? = .None;
    let _ = println(x ?? 99);
    0
}
