// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a: Bool? = null;
    let b: Bool = false;
    let c: Bool? = .Some(true);
    let d: Bool = true;
    let e: Bool = true;
    // (null ?? false) or (Some(true) ?? true and true)
    // = false or (true and true)
    // = false or true
    // = true
    let _ = println(a ?? b or c ?? d and e);
    0
}
