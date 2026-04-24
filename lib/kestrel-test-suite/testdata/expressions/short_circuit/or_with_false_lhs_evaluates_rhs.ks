// test: diagnostics

module Main
import std.io.stdio.println

func sideEffect() -> Bool {
    let _ = println("RHS");
    true
}

func main() -> lang.i64 {
    let result = false or sideEffect();
    let _ = println(result);
    0
}
