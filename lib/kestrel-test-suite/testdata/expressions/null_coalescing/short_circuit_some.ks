// test: diagnostics

module Main
import std.streams.println

func getDefault() -> Int {
     println("DEFAULT");
    99
}

func main() -> lang.i64 {
    let a: Int? = .Some(1);
    let result = a ?? getDefault();
     println(result);
    0
}
