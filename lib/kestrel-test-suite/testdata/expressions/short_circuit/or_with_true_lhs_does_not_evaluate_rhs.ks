// test: diagnostics

module Main
import std.streams.println

func sideEffect() -> Bool {
     println("RHS");
    false
}

func main() -> lang.i64 {
    let result = true or sideEffect();
     println(result);
    0
}
