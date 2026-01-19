// Minimal hello world to test string output.

module Hello

import io.stdio.(println)
import io.error.(Error)
import std.result.(Result)

func main() -> Result[(), Error] {
    println("Hello");
    .Ok(())
}
