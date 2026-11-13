// test: diagnostics

module Main
import std.streams.println

func getOrDefault(opt: Int?) -> Int {
    opt ?? 42
}

func main() -> lang.i64 {
     println(getOrDefault(.Some(1)));
     println(getOrDefault(null));
    0
}
