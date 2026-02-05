module Test

func main() -> lang.i64 {
    var arr = std.collections.Array[std.num.Int64]();
    arr.append(1);
    arr.append(2);
    arr.append(3);

    // Test scan (running sum)
    let running: std.collections.Array[std.num.Int64] = arr.iter().scan(0, { (acc, x) in acc + x }).collect();
    if running.count != 3 { return 3 }
    if running(unchecked: 0) != 1 { return 4 }
    if running(unchecked: 2) != 6 { return 5 }

    // Test position
    let pos = arr.iter().position({ (x) in x == 2 });
    if pos.isNone() { return 6 }
    if pos.unwrap() != 1 { return 7 }

    // Test contains
    if arr.iter().contains(2) == false { return 8 }
    if arr.iter().contains(10) { return 9 }

    0
}
