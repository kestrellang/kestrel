module Test

func main() -> lang.i64 {
    var arr = std.collections.Array[std.num.Int64]();
    arr.append(1);
    arr.append(2);

    let flat = arr.iter().map({ (x) in x * 2 }).collect();

    // This should work - flat is Array[Int64], so flat(unchecked: 0) should be Int64
    if flat(unchecked: 0) != 2 { return 1 }

    0
}
