module Test

func main() -> lang.i64 {
    // Test stepBy
    let everyOther: std.collections.Array[std.num.Int64] = [0, 1, 2, 3, 4, 5, 6].iter().stepBy(2).collect();
    if everyOther.count != 4 { return 1 }
    if everyOther(unchecked: 1) != 2 { return 2 }

    0
}
