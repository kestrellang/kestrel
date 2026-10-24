// test: execution
// stdlib: true
// expect-exit: 0
//
// A consuming requirement through `any[OwnedBox] P` moves the payload out;
// its deinit runs exactly once (design §3, ConsumingIndirection).

module Test

import std.numeric.(Int64)
import std.memory.(OwnedBox)

var drops: Int64 = 0;

protocol Closeable {
    consuming func close() -> Int64
}

struct File: not Copyable {
    var fd: Int64;
    deinit { drops = drops + 1; }
}
extend File: Closeable {
    public consuming func close() -> Int64 { self.fd }
}

func run() -> Int64 {
    let f: any[OwnedBox] Closeable = File(fd: 7);
    let fd = f.close();
    if fd != 7 { return 1 }
    if drops != 1 { return 2 }
    0
}

@main
func main() -> Int64 {
    let r = run();
    if r != 0 { return r }
    if drops != 1 { return 3 }
    0
}
