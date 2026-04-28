// test: execution
// expect-stdout: 1\n

module Main
import std.io.stdio.println

func doNothing() -> () = ()

func main() -> std.numeric.Int64 {
    doNothing();
    let _ = println(1);
    0
}
