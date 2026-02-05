module Test

func main() -> lang.i64 {
    // This should work - direct array count
    var arr1 = std.collections.Array[std.num.Int64]();
    arr1.append(1);
    if arr1.count != 1 { return 1 }

    // This fails - count on collected array
    let arr2 = [1, 2, 3].iter().collect();
    if arr2.count != 3 { return 2 }

    0
}
