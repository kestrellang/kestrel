// test: execution
// stdlib: true
// backends: cranelift,llvm

// Cross-mix `ref` + `set`: reads borrow in place through the ref
// accessor; writes go through the set hook (observable side effect —
// a write counter). RMW has no `mutating ref`, so it falls back to
// writeback: read through REF, mutate the copy, write through SET (the
// counter ticks).
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Logged {
    var p: Pointer[Int64]
    var writes: Pointer[Int64]

    subscript(at index: Int64) -> Int64 {
        ref { unsafe self.p.offset(by: index).value }
        set {
            unsafe self.writes.write(self.writes.read() + 1);
            unsafe self.p.offset(by: index).write(newValue);
        }
    }
}

@main
func main() -> lang.i64 {
    let p = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    let w = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    unsafe p.write(5);
    unsafe w.write(0);
    var l = Logged(p: p, writes: w);

    // Reads: through the ref accessor, no set-hook ticks.
    if l(at: 0) != 5 { return 1; }
    if unsafe w.read() != 0 { return 2; }

    // Write: through the set hook.
    l(at: 0) = 9;
    if l(at: 0) != 9 { return 3; }
    if unsafe w.read() != 1 { return 4; }

    // RMW: ref-read → op → set writeback; the hook ticks once.
    l(at: 0) += 3;
    if l(at: 0) != 12 { return 5; }
    if unsafe w.read() != 2 { return 6; }
    0
}
