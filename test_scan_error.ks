module Test

func main() -> lang.i64 {
    var arr = std.collections.Array[std.num.Int64]();
    arr.append(1);
    arr.append(2);
    arr.append(3);

    // Test scan (running sum) - this is likely causing the error
    let running: std.collections.Array[std.num.Int64] = arr.iter().scan(0, { (acc, x) in acc + x }).collect();
    if running.count != 3 { return 3 }
    if running(unchecked: 0) != 1 { return 4 }
    if running(unchecked: 2) != 6 { return 5 }

    0
}
