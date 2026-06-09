// test: execution
// stdlib: true
// backends: cranelift,llvm
// skip: stage1 — needs Pointer.value (S3)

// NotCopyable pointee through a borrow-convention argument: compiles
// (misclassification as value context would fail the copy guards at
// compile time) and deinits exactly once.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Res: not Copyable {
    var cell: Pointer[Int64]
    func current() -> Int64 { self.cell.read() }
    deinit { self.cell.write(self.cell.read() - 1); }
}

func inspect(r: Res) -> Int64 { r.current() }

func run(counter: Pointer[Int64]) -> Int64 {
    let r = Res(cell: counter);
    let p = Pointer(to: r);
    // ref → borrow param: the place passes, the resource never moves
    if inspect(p.value) != 10 { return 1; }
    if r.current() != 10 { return 2; }
    0
}

@main
func main() -> lang.i64 {
    let counter = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    counter.write(10);

    let status = run(counter);
    if status != 0 { return status.raw; }

    if counter.read() != 9 { return 3; }
    0
}
