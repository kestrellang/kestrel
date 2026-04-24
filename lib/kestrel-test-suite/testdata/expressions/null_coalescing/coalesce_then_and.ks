// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let x: Bool? = null;
    let y: Bool = false;
    // (null ?? true) and false = true and false = false
    let _ = println(x ?? true and y);
    0
}
