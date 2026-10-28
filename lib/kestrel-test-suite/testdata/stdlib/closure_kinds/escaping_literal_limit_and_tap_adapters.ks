// test: execution
// stdlib: true
// expect-exit: 0

// Second half of the "escaping literal through every lazy builder" row of the
// closures-stdlib-audit.md test matrix: takeWhile, skipWhile, inspect and
// intersperseWith. Also pins the escaping body's *own* mutable state: an
// escaping closure mutates its snapshot, which persists across calls and is
// invisible to the frame it snapshotted (docs/design/closures.md, "escaping").
module Test

public var visits: Int64 = 0;

@main
func main() -> lang.i64 {
    // takeWhile — snapshot of `limit`
    var limit: Int64 = 4;
    let head = [1, 2, 3, 4, 5].iter().takeWhile(where: { (x) in x < limit });
    limit = 0;
    let headOut: Array[Int64] = head.collect();
    if headOut.count != 3 { return 1 }
    if headOut(unchecked: 0) != 1 { return 2 }
    if headOut(unchecked: 2) != 3 { return 3 }

    // skipWhile — snapshot of `lower`
    var lower: Int64 = 3;
    let tail = [1, 2, 3, 4].iter().skipWhile(where: { (x) in x < lower });
    lower = 100;
    let tailOut: Array[Int64] = tail.collect();
    if tailOut.count != 2 { return 4 }
    if tailOut(unchecked: 0) != 3 { return 5 }
    if tailOut(unchecked: 1) != 4 { return 6 }

    // inspect — elements flow through unchanged; the escaping tap records into
    // a module-level var (not a capture, so no write-back is required).
    var stride: Int64 = 10;
    let tapped = [1, 2, 3].iter().inspect({ (x) in visits = visits + x * stride });
    stride = 0;
    let tappedOut: Array[Int64] = tapped.collect();
    if tappedOut.count != 3 { return 7 }
    if tappedOut(unchecked: 2) != 3 { return 8 }
    if visits != 60 { return 9 }

    // intersperseWith — snapshot of `sep`
    var sep: Int64 = 0;
    let woven = [1, 2, 3].iter().intersperseWith(with: { () in sep });
    sep = 9;
    let wovenOut: Array[Int64] = woven.collect();
    if wovenOut.count != 5 { return 10 }
    if wovenOut(unchecked: 1) != 0 { return 11 }
    if wovenOut(unchecked: 3) != 0 { return 12 }

    // The escaping environment is the closure's own state: mutating it across
    // calls is visible only inside, and the snapshotted source var stays put.
    var counter: Int64 = 0;
    let numbered = [1, 2, 3].iter().intersperseWith(with: { () in counter = counter + 10; counter });
    let numberedOut: Array[Int64] = numbered.collect();
    if numberedOut.count != 5 { return 13 }
    if numberedOut(unchecked: 1) != 10 { return 14 }
    if numberedOut(unchecked: 3) != 20 { return 15 }
    if counter != 0 { return 16 }

    0
}
