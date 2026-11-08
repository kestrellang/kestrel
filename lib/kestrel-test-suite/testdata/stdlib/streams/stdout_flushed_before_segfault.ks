// test: execution
// stdlib: true
// expect-exit: -1
// expect-stdout: before crash\n

module Test

import std.memory.(RawPointer)

// A segfault kills the process before `atexit` runs. The fatal-signal
// handler flushes buffered standard output first, so this line survives.
@main
func main() {
    println("before crash");
    let p = RawPointer.nullPointer();
    let v = unsafe p.cast[Int64]().read();
    println("\(v)");
}
