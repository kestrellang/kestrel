// test: diagnostics

module Main
import std.io.stdio.println

func sideEffect() -> Bool {
    let _ = println("RHS");
    false
}

func main() -> lang.i64 {
    let result = true or sideEffect();
    let _ = println(result);
    0
}
