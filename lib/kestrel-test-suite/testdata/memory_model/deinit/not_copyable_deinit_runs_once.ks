// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: moved 7\nclose 7\n

module Test

import std.numeric.Int64
import std.io.stdio.println

// With `not Copyable`, `let b = a` is a move, so the deinit runs exactly once.
struct L: not Copyable {
    var fd: Int64;
    deinit { println("close \(self.fd)"); }
}

@main
func main() {
    let a = L(fd: 7);
    let b = a;
    println("moved \(b.fd)");
}
