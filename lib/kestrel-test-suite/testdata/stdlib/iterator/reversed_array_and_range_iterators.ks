// test: execution
// stdlib: true
// expect-stdout: [5, 4, 3, 2, 1] [3, 2, 1, 0] [3, 2, 1] [1, 2, 3]\nSome(1) Some(3) Some(2) None\nSome(4) Some(1) Some(3) Some(2) None\n1210\n

// Array, Range and ClosedRange iterators are double-ended, so `.reversed()`
// works on them, and `next()`/`nextBack()` meet in the middle.
module Test

@main
func main() -> Int64 {
    let a = [1, 2, 3, 4, 5].iter().reversed().collect();
    let b = (0..<4).iter().reversed().collect();
    let c = (1..=3).iter().reversed().collect();
    let r = [1, 2, 3].iter().reversed().reversed().collect();
    println("\(a) \(b) \(c) \(r)");

    var it = [1, 2, 3].iter();
    let f = it.next();
    let k = it.nextBack();
    let m = it.next();
    let n = it.nextBack();
    println("\(f) \(k) \(m) \(n)");

    var cr = (1..=4).iter();
    let c1 = cr.nextBack();
    let c2 = cr.next();
    let c3 = cr.nextBack();
    let c4 = cr.nextBack();
    let c5 = cr.next();
    println("\(c1) \(c2) \(c3) \(c4) \(c5)");

    // Single-element and empty closed ranges, and a half-open range.
    var s = 0;
    for x in (1..=1).iter().reversed() { s = s * 10 + x; }
    for x in (3..=1).iter().reversed() { s = s * 10 + x; }
    for x in (0..<3).iter().reversed() { s = s * 10 + x; }
    println(s);
    0
}
