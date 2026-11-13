// test: execution
// stdlib: true
// expect-exit: -1

// `println` returns `()`: a failed write to standard output can't be
// ignored by accident, so it stops the process with a fatal error instead
// (as Rust's `println!` panics). Here fd 1 is closed, so once the buffer
// fills, the write fails with EBADF and the program traps before `return`.
module Test

@extern(.C)
func close(fd: Int32) -> Int32

@main
func main() -> Int64 {
    let unit: () = println("before");
    unsafe close(1);
    var i = 0;
    while i < 100000 {
        println("line \(i)");
        i += 1
    }
    0
}
