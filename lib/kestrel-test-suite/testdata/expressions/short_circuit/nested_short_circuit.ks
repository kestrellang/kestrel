// test: diagnostics

module Main
import std.io.stdio.println

func effect() -> Bool {
     println("EFFECT");
    true
}

func main() -> lang.i64 {
    let result = (false and (effect() and effect())) or true;
     println(result);
    0
}
