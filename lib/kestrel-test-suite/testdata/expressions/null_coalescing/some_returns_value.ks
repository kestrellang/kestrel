// test: diagnostics

module Main
import std.io.stdio.println
import std.result.Optional

func main() -> lang.i64 {
    let x: Int? = .Some(42);
    let _ = println(x ?? 0);
    0
}
