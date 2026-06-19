// test: execution
// backends: cranelift,llvm
// stdlib: true

// Returning a STORED `&T` field from a ret_borrow function must yield the
// stored ref (a load of the slot), not a borrow OF the slot — the old
// return fast-path projected the slot's address and the caller read the
// pointer's bits as the value (printed an address, not 42). The place
// resolver routes ref slots through the loaded-ref view, rooted at the
// aggregate (Param here), so the return is legal and aliases correctly.
module Test

struct Cursor {
    var item: &Int64
}

func fetch(mutating c: Cursor) -> &Int64 {
    c.item
}

@main
func main() -> lang.i64 {
    var x = 42;
    let r = &x;
    var c = Cursor(item: r);
    let v = fetch(c);
    if v != 42 { return 1; }
    x = 9;
    if fetch(c) != 9 { return 2; }
    0
}
