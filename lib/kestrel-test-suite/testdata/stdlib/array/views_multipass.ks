// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var arr = std.collections.Array[std.numeric.Int64]();
    arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);

    // ChunksView is multi-pass: count + indexed access without iterating.
    let chunks = arr.chunks(of: 2);
    if chunks.count != 3 { return 1 }
    if chunks(0).count != 2 { return 2 }
    if chunks(0)(unchecked: 0) != 1 { return 3 }
    if chunks(2).count != 1 { return 4 }
    if chunks(2)(unchecked: 0) != 5 { return 5 }

    // Multi-pass: iterate, then iterate again.
    var firstPassSum: std.numeric.Int64 = 0;
    for c in chunks {
        firstPassSum = firstPassSum + c.count
    }
    if firstPassSum != 5 { return 6 }
    var secondPassSum: std.numeric.Int64 = 0;
    for c in chunks {
        secondPassSum = secondPassSum + c.count
    }
    if secondPassSum != 5 { return 7 }

    // WindowsView is multi-pass with O(1) count.
    let windows = arr.windows(of: 3);
    if windows.count != 3 { return 8 }
    if windows(0)(unchecked: 0) != 1 { return 9 }
    if windows(2)(unchecked: 2) != 5 { return 10 }

    // ReversedView keeps the source intact.
    let rev = arr.reversed();
    if rev.count != 5 { return 11 }
    if rev(0) != 5 { return 12 }
    if rev(4) != 1 { return 13 }
    if arr(unchecked: 0) != 1 { return 14 }

    // ArraySplitView via toArray().
    var arr2 = std.collections.Array[std.numeric.Int64]();
    arr2.append(1); arr2.append(0); arr2.append(2); arr2.append(0); arr2.append(3);
    let parts = arr2.split(0).toArray();
    if parts.count != 3 { return 15 }

    // Eager map/filter via Slice extension.
    let doubled = arr.map { it * 2 };
    if doubled.count != 5 { return 16 }
    if doubled(unchecked: 0) != 2 { return 17 }
    if doubled(unchecked: 4) != 10 { return 18 }

    let evens = arr.filter(matching: { (x) in x % 2 == 0 });
    if evens.count != 2 { return 19 }
    if evens(unchecked: 0) != 2 { return 20 }
    if evens(unchecked: 1) != 4 { return 21 }

    0
}
