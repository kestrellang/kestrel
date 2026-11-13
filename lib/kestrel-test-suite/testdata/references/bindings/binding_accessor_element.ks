// test: execution
// stdlib: true
// backends: cranelift,llvm

// Named bindings over place-accessor elements: `&arr(i)` holds the
// element place (read through the `ref` accessor); `&mutating arr(i)`
// holds the mutable place (the `mutating ref` accessor) and stores write
// the element in place. Cross-feature pin: item 1's accessors × item 2's
// bindings. Ref work straight-line; asserts after (bindings are
// block-local).
module Test

import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    var arr = [10, 20, 30];
    let e = &arr(1);
    let v1 = e;

    let m = &mutating arr(2);
    m = 99;
    let v2 = arr(2);
    m = m + 1;
    let v3 = arr(2);

    if v1 != 20 { return 1; }
    if v2 != 99 { return 2; }
    if v3 != 100 { return 3; }
    0
}
