// test: diagnostics

module Main
import std.io.stdio.println

func sideEffect() -> Bool {
     println("RHS");
    true
}

func main() -> lang.i64 {
    let result = false and sideEffect();
     println(result);
    0
}
