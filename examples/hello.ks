// Minimal hello world to test string output.

module Hello

import std.io.stdio.(println)
import std.io.error.(Error)
import std.result.(Result)

func main() -> Result[(), Error] {
    println("Hello");
    .Ok(())
}
