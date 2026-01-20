// Minimal reproduction for codegen bug: load.i8 with i8 type address
// Bug triggers when: struct method with nested loops + if/else + print before/after

module BugRepro

import io.stdio.(print, println)
import io.error.(Error)
import std.result.(Result)
import std.num.(Int64)
import std.core.(Bool)

struct Pong {
    var width: Int64
    var height: Int64

    init() {
        self.width = Int64(intLiteral: 10);
        self.height = Int64(intLiteral: 10);
    }

    func render() -> Result[(), Error] {
        var y: Int64 = Int64(intLiteral: 0);
        while y < self.height {
            print("A");
            var x: Int64 = Int64(intLiteral: 0);
            while x < self.width {
                if true {
                    print("B");
                } else {
                    print("C");
                }
                x = x + Int64(intLiteral: 1);
            }
            println("D");
            y = y + Int64(intLiteral: 1);
        }
        .Ok(())
    }
}

func main() {
    var g = Pong();
    g.render();
}
