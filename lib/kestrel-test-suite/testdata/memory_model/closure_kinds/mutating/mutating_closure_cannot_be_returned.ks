// test: diagnostics
// stdlib: false

// docs/design/closures.md: `mutating` is a view kind — its environment holds
// &mutating views of the frame — so like a `mutating` method it cannot leave
// its frame. Returning one is the ordinary provenance escape error (E494),
// whose fix-it points at an owning kind (`escaping` / `consuming`).
// MIR-stage code: this file must stay free of analyzer-stage errors.
module Test

func makeBumper() -> mutating () -> () {
    var total = 0;
    let bump: mutating () -> () = { total = lang.i64_add(total, 1); };
    bump // ERROR(E494)
}
