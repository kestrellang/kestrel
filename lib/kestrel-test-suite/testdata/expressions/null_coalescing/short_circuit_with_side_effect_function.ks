// test: diagnostics

module Main
import std.io.stdio.println

func getDefault() -> Int {
     println("called");
    999
}

func main() -> lang.i64 {
    let x: Int? = .Some(10);
    let y: Int? = null;

    // x is Some, so getDefault should NOT be called
    let a = x ?? getDefault();
     println(a);

    // y is None, so getDefault SHOULD be called
    let b = y ?? getDefault();
     println(b);
    0
}
