// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println((true and false) or (false and true) or (true and true));
    0
}
