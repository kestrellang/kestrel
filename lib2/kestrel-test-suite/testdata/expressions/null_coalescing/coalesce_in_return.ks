// test: diagnostics

module Main
import std.io.stdio.println

func getOrDefault(opt: Int?) -> Int {
    opt ?? 42
}

func main() -> lang.i64 {
    let _ = println(getOrDefault(.Some(1)));
    let _ = println(getOrDefault(null));
    0
}
