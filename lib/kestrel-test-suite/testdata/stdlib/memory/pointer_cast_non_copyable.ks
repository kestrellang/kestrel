// test: execution
// stdlib: true

module Test

// A unique, owning resource — never bit-copyable.
struct Res: not Copyable {
    var fd: lang.i64
}

@main
func main() -> lang.i64 {
    // A `Pointer` to a non-Copyable type is legitimate: the pointer is
    // just a machine address. `cast` reinterprets the address; it never
    // bit-copies the pointee. This must compile and run.
    var alloc = std.memory.SystemAllocator();
    let layout = std.memory.Layout(size: 8, alignment: 8);
    let result = alloc.allocate(layout);
    if result.isNone() { return 1 }
    let raw = result.unwrap();
    let typed: std.memory.Pointer[Res] = raw.cast[Res]();
    if typed.isNull { return 2 }
    alloc.deallocate(raw, layout);
    0
}
