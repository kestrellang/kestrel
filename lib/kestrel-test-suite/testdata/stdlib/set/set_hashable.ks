// test: execution
// stdlib: true

// Set conditional Hashable conformance: order-INDEPENDENT — sets with the
// same elements hash equal regardless of insertion order; different sets
// hash differently.

module Test

@main
func main() -> lang.i64 {
    var a = std.collections.Set[std.numeric.Int64]();
    a.insert(1);
    a.insert(2);
    a.insert(3);

    var b = std.collections.Set[std.numeric.Int64]();
    b.insert(3);
    b.insert(1);
    b.insert(2);

    var c = std.collections.Set[std.numeric.Int64]();
    c.insert(1);
    c.insert(2);
    c.insert(4);

    var ha = std.collections.DefaultHasher();
    a.hash(into: ha);
    let da = ha.finish();

    var hb = std.collections.DefaultHasher();
    b.hash(into: hb);
    let db = hb.finish();

    var hc = std.collections.DefaultHasher();
    c.hash(into: hc);
    let dc = hc.finish();

    // Same elements, different insertion order: must hash equal.
    if da != db { return 1 }
    // Different elements: should differ.
    if da == dc { return 2 }

    0
}
