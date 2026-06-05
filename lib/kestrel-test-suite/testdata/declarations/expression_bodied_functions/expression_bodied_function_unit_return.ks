// test: execution
// expect-stdout: 1\n

module Main
import std.io.stdio.println

func doNothing() -> () = ()

@main
func main() -> lang.i64 {
    doNothing();
     println(1);
    0
}
