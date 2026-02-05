module Test

func main() -> lang.i64 {
    var arr = std.collections.Array[std.num.Int64]();
    arr.append(1);
    arr.append(2);
    arr.append(3);

    // Test stepBy - this is likely causing the error
    let everyOther: std.collections.Array[std.num.Int64] = [0, 1, 2, 3, 4, 5, 6].iter().stepBy(2).collect();
    if everyOther.count != 4 { return 1 }

    0
}
