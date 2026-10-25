// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: end\nholder deinit\nclose fd 9\nenum deinit closed\nenum deinit open 3\n

module Test

import std.numeric.Int64
import std.io.stdio.println

struct Fd: not Copyable {
    var n: Int64;
    deinit { println("close fd \(self.n)"); }
}

// A move-only enum may declare a deinit; it can match on its own case.
enum Res: not Copyable {
    case open(Int64)
    case closed
    deinit {
        match self {
            .open(n) => println("enum deinit open \(n)"),
            .closed => println("enum deinit closed"),
        }
    }
}

// User deinit runs before the payload's own deinit.
enum Holder: not Copyable {
    case held(Fd)
    case empty
    deinit { println("holder deinit"); }
}

@main
func main() {
    let a = Res.open(3);
    let b = a; // move: deinit still runs exactly once
    let c = Res.closed;
    let h = Holder.held(Fd(n: 9));
    println("end");
}
