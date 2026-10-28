// test: execution
// stdlib: true

// Dictionary conditional Hashable conformance: order-independent over
// (key, value) entries.

module Test

@main
func main() -> lang.i64 {
    var a: [String: Int64] = [:];
    a.insert("x", 1);
    a.insert("y", 2);

    var b: [String: Int64] = [:];
    b.insert("y", 2);
    b.insert("x", 1);

    // Same keys, different value: must differ.
    var c: [String: Int64] = [:];
    c.insert("x", 1);
    c.insert("y", 3);

    // Key/value swap-shaped difference.
    var d: [String: Int64] = [:];
    d.insert("x", 2);
    d.insert("y", 1);

    var ha = std.collections.DefaultHasher();
    a.hash(into: ha);
    let da = ha.finish();

    var hb = std.collections.DefaultHasher();
    b.hash(into: hb);
    let db = hb.finish();

    var hc = std.collections.DefaultHasher();
    c.hash(into: hc);
    let dc = hc.finish();

    var hd = std.collections.DefaultHasher();
    d.hash(into: hd);
    let dd = hd.finish();

    // Equal dictionaries (insertion order differs): equal hashes.
    if da != db { return 1 }
    if da == dc { return 2 }
    if da == dd { return 3 }

    0
}
