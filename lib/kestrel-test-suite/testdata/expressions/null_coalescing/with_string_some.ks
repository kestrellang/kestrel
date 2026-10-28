// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let name: String? = .Some("Alice");
     println(name ?? "anonymous");
    0
}
