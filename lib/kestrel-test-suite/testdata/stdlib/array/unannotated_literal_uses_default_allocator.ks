// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 4 a\n

// An unannotated array literal defaults to `Array[T, GlobalAllocator]`: the
// allocator param's declared default fills in when nothing pins it.
module Test

@main
func main() {
    let xs = [1, 2, 3, 4];
    let ss = ["a"];
    let first = ss(0);
    print("\(xs.count) \(first)\n");
}
