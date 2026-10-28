// test: execution
// stdlib: true

// Array conditional Hashable conformance: order-sensitive — equal arrays
// hash equal, differently-ordered arrays (which are != ) hash differently.

module Test

@main
func main() -> lang.i64 {
    let a = [1, 2, 3];
    let b = [1, 2, 3];
    let c = [3, 2, 1];

    var ha = std.collections.DefaultHasher();
    a.hash(into: ha);
    let da = ha.finish();

    var hb = std.collections.DefaultHasher();
    b.hash(into: hb);
    let db = hb.finish();

    var hc = std.collections.DefaultHasher();
    c.hash(into: hc);
    let dc = hc.finish();

    // Equal arrays must hash equal.
    if da != db { return 1 }
    // Order-sensitive: reversed contents should differ (not guaranteed in
    // theory, but a collision here would indicate the count/element feed
    // is broken).
    if da == dc { return 2 }

    // Count prefix disambiguates nesting: [[1], [2]] vs [[1, 2], []].
    let n1 = [[1], [2]];
    let n2 = [[1, 2], []];
    var h1 = std.collections.DefaultHasher();
    n1.hash(into: h1);
    var h2 = std.collections.DefaultHasher();
    n2.hash(into: h2);
    if h1.finish() == h2.finish() { return 3 }

    0
}
