// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(not false or true);  // (not false) or true = true or true = true
    0
}
