// test: execution
// stdlib: true
// expect-exit: 0

// A place on the left is substituted directly, so a borrowing callee
// borrows it: a non-Copyable value is still usable afterwards.

module Test

struct Handle: not Copyable {
    var id: Int64
}

struct Holder: not Copyable {
    var handle: Handle
}

func size(h: Handle) -> Int64 { h.id * 10 }
func plus(h: Handle, n: Int64) -> Int64 { h.id + n }

@main
func main() -> lang.i64 {
    let h = Handle(id: 3);
    if (h |> size) != 30 { return 1 }
    if (h |> plus(_, 1)) != 4 { return 2 }
    // still owned here
    if h.id != 3 { return 3 }
    // a field place
    let holder = Holder(handle: Handle(id: 5));
    if (holder.handle |> size) != 50 { return 4 }
    if holder.handle.id != 5 { return 5 }
    0
}
