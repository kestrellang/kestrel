// test: execution
// stdlib: true
// backends: cranelift,llvm

// The strongest no-clone pin: a NotCopyable pointee. If the borrowed-self
// call through `&Res` were misclassified as a value context, the copy
// guards would reject this AT COMPILE TIME — compiling and running with an
// exact deinit count proves the place path.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Res: not Copyable {
    var cell: Pointer[Int64]
    func current() -> Int64 { self.cell.read() }
    deinit { self.cell.write(self.cell.read() - 1); }
}

func run(counter: Pointer[Int64]) -> Int64 {
    let r = Res(cell: counter);
    let p = Pointer(to: r);
    // borrowed-self method through the Pointer-derived ref — no move, no copy
    if p.value.current() != 10 { return 1; }
    if r.current() != 10 { return 2; }
    0
}

@main
func main() -> lang.i64 {
    let counter = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    counter.write(10);

    let status = run(counter);
    if status != 0 { return status.raw; }

    // exactly one deinit, at `run` scope exit
    if counter.read() != 9 { return 3; }
    0
}
