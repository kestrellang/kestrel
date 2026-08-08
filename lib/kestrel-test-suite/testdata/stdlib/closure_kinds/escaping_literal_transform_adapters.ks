// test: execution
// stdlib: true
// expect-exit: 0

// The lazy builders store their callback, so their parameters take `escaping`
// closures (closures-stdlib-audit.md, "escaping: Callbacks Stored for Later
// Use"). An escaping literal owns *snapshots* of its captures, so mutating the
// source var after the adapter is built cannot change the adapter's output
// (docs/design/closures.md, "Capture Rules"). Covers map/filter/filterMap/
// flatMap/scan.
module Test

@main
func main() -> lang.i64 {
    // map
    var factor: Int64 = 2;
    let doubled = [1, 2, 3].iter().map(as: { (x) in x * factor });
    factor = 100;
    let doubledOut: Array[Int64] = doubled.collect();
    if doubledOut.count != 3 { return 1 }
    if doubledOut(unchecked: 0) != 2 { return 2 }
    if doubledOut(unchecked: 2) != 6 { return 3 }

    // filter
    var threshold: Int64 = 2;
    let big = [1, 2, 3, 4].iter().filter(where: { (x) in x > threshold });
    threshold = 100;
    let bigOut: Array[Int64] = big.collect();
    if bigOut.count != 2 { return 4 }
    if bigOut(unchecked: 0) != 3 { return 5 }
    if bigOut(unchecked: 1) != 4 { return 6 }

    // filterMap
    var scale: Int64 = 10;
    let evens = [1, 2, 3, 4].iter().filterMap(as: { (x) in if x % 2 == 0 { .Some(x * scale) } else { .None } });
    scale = 0;
    let evensOut: Array[Int64] = evens.collect();
    if evensOut.count != 2 { return 7 }
    if evensOut(unchecked: 0) != 20 { return 8 }
    if evensOut(unchecked: 1) != 40 { return 9 }

    // flatMap
    let nested = [[1, 2], [3, 4]];
    var skipCount: Int64 = 1;
    let flat = nested.iter().flatMap(as: { (inner) in inner.iter().skip(skipCount) });
    skipCount = 0;
    let flatOut: Array[Int64] = flat.collect();
    if flatOut.count != 2 { return 10 }
    if flatOut(unchecked: 0) != 2 { return 11 }
    if flatOut(unchecked: 1) != 4 { return 12 }

    // scan
    var mult: Int64 = 2;
    let running = [1, 2, 3].iter().scan(from: 0, by: { (acc, x) in acc + x * mult });
    mult = 0;
    let runningOut: Array[Int64] = running.collect();
    if runningOut.count != 3 { return 13 }
    if runningOut(unchecked: 0) != 2 { return 14 }
    if runningOut(unchecked: 1) != 6 { return 15 }
    if runningOut(unchecked: 2) != 12 { return 16 }

    0
}
