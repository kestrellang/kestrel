// test: execution
// stdlib: true

// NOTE (2026-06-01): currently FAILS at runtime (crash), but NOT because of the
// clone shim — the whole `Array` execution path crashes today due to the
// in-progress non-Copyable var-read move-lowering (see memory
// `mir3_noncopyable_var_read_copy`; existing `stdlib.array.*` execution tests
// crash identically). This test is the deep-clone regression guard for
// conditional containers (gap #2): once Array execution works it should pass,
// proving `Optional[Array]`'s clone shim deep-copies rather than aliasing.

module Test

// `Optional[Array[Int64]]` is a conditional container resolving to `Clone`
// (Array is Cloneable). Copying it must DEEP-clone — a bit-copy would alias
// the Array's COW storage without bumping its refcount, so mutating one copy
// would corrupt the other (and double-free at teardown). This fails (1) if the
// clone shim aliases instead of deep-copying.
func main() -> lang.i64 {
    let opt: std.result.Optional[std.collections.Array[lang.i64]] = .Some([10, 20, 30]);
    let opt2 = opt;            // must clone (Optional[Array] is Cloneable)

    var a = opt.unwrap();
    var b = opt2.unwrap();

    a.append(99);              // mutate only `a`

    if b.count != 3 { return 1; }   // `b` must be untouched by `a`'s mutation
    if a.count != 4 { return 2; }   // sanity: `a` did grow
    return 0
}
