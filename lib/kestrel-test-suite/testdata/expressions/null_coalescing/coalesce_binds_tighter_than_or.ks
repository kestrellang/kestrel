// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let x: Bool? = null;
    let y: Bool = true;
    // (null ?? false) or true = false or true = true
     println(x ?? false or y);
    0
}
