// test: execution
// stdlib: true
// expect-exit: -1

// #127 backstop: reading a `not Copyable` element out of an array BY VALUE
// (`rs(0).tag` routes through `Slice.subscript` -> `Pointer.read()`, whose
// `where T: Copyable` bound the frontend fails to enforce for
// Copyable-default type params) used to bit-copy the element and deinit one
// logical value twice — silent heap corruption. Monomorphization now poisons
// any instantiation that violates an explicit `T: Copyable` bound with a
// non-Copyable concrete type: executing it traps instead of corrupting.
// Statically-reachable-but-unexecuted instantiations (e.g. Array[T]'s clone
// shim) still compile — see array_literal_notcopyable_no_spurious_deinit.ks
// for the benign path.
//
// When the real fix lands (borrow-returning subscript reads, or
// instantiation-site Copyable enforcement), this program should either
// compile-and-run correctly (drop count 1, exit 0) or be rejected at compile
// time — either way this trap expectation must be revisited.
//
// See lib/kestrel-mir/src/mono/mod.rs::violated_copyable_bound.

module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Res: not Copyable {
    var tag: Int64
    var drops: Pointer[Int64]
    deinit { self.drops.write(self.drops.read() + 1); }
}

@main
func main() -> lang.i64 {
    let drops = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    drops.write(0);
    var rs = [Res(tag: 1, drops: drops)];
    // Bit-copies the non-Copyable element out of the array — must trap.
    let t = rs(0).tag;
    t.raw
}
