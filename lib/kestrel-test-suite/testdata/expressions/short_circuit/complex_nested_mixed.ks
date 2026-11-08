// test: diagnostics

module Main
import std.streams.println

func main() -> lang.i64 {
     println((true and false) or (false and true) or (true and true));
    0
}
