module Test

func main() -> lang.i64 {
    var arr = std.collections.Array[std.num.Int64]();
    arr.append(3);
    arr.append(1);
    arr.append(4);
    arr.append(1);
    arr.append(5);

    // Test min
    let minVal = arr.iter().min();
    if minVal.isNone() { return 1 }
    if minVal.unwrap() != 1 { return 2 }

    // Test max
    let maxVal = arr.iter().max();
    if maxVal.isNone() { return 3 }
    if maxVal.unwrap() != 5 { return 4 }

    0
}
