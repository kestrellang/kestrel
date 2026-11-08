// test: execution
// expect-stdout: 1\n

module Main
import std.streams.println

func doNothing() -> () = ()

@main
func main() -> lang.i64 {
    doNothing();
     println(1);
    0
}
