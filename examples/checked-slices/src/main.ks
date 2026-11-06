module CheckedSlices

/// Sum of a slice. Takes any slice, of any array.
func sum(s: ArraySlice[Int64]) -> Int64 {
    var total: Int64 = 0;
    for i in 0..<s.count {
        total += s(i);
    }
    total
}

/// The longest run of increasing elements, as a slice of the caller's array.
/// Returning a slice of a parameter is fine: it borrows the caller's array.
func longestRun(xs: [Int64]) -> ArraySlice[Int64] {
    var bestStart: Int64 = 0;
    var bestCount: Int64 = if xs.isEmpty { 0 } else { 1 };
    var start: Int64 = 0;
    for i in 1..<xs.count {
        if xs(i) <= xs(i - 1) {
            start = i;
        }
        if i - start + 1 > bestCount {
            bestStart = start;
            bestCount = i - start + 1;
        }
    }
    xs.slice(from: bestStart, count: bestCount)
}

func show(label: String, s: ArraySlice[Int64]) {
    var text = "";
    for i in 0..<s.count {
        if i > 0 { text = text + ", "; }
        text = text + "\(s(i))";
    }
    println("\(label): [\(text)]");
}

@main
func main() {
    var xs: [Int64] = [9, 4, 7, 1, 8, 2, 6];

    // Read-only slices borrow the array; narrowing keeps the same base.
    let middle = xs.slice(from: 1, count: 5);
    show("middle", middle);
    println("sum of middle: \(sum(middle))");
    show("middle.slice(1, 2)", middle.slice(from: 1, count: 2));

    // Slices returned from helpers borrow the caller's array.
    show("longest increasing run", longestRun([3, 1, 2, 5, 9, 4]));

    // Growing the array while a slice is live is safe: the slice reads
    // through the array, so it sees the new buffer.
    let head = xs.slice(from: 0, count: 3);
    for i in 0..<100 {
        xs.append(i);
    }
    show("head after 100 appends", head);

    // Mutable slices write in place, through the array's copy-on-write check.
    let snapshot = xs;
    let part = xs.mutatingSlice(from: 0, count: 7);
    part.sort();
    show("sorted first 7", part.readOnly());
    println("snapshot unchanged: \(snapshot(0)) \(snapshot(1)) \(snapshot(2))");

    let tail = xs.mutatingSlice(from: 4, count: 3);
    tail.reverse();
    tail.update(at: 0, to: 100);
    show("first 7 after reversing [4, 7) and setting [4]", xs.slice(from: 0, count: 7));

    xs.mutatingSlice(from: 0, count: 2).fill(with: 0);
    show("after fill", xs.slice(from: 0, count: 7));

    // These are compile errors:
    //
    //   func leak() -> ArraySlice[Int64] {
    //       let local: [Int64] = [1, 2, 3];
    //       local.slice(from: 0, count: 2)   // E494: borrows local `local`
    //   }
    //
    //   var outer = xs.slice(from: 0, count: 1);
    //   if true {
    //       let inner: [Int64] = [1];
    //       outer = inner.slice(from: 0, count: 1);   // E509: outlives `inner`
    //   }
}
