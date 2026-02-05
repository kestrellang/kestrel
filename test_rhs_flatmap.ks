module Test

func main() -> lang.i64 {
    var nested = std.collections.Array[std.collections.Array[std.num.Int64]]();
    var inner1 = std.collections.Array[std.num.Int64]();
    inner1.append(1);
    nested.append(inner1);

    // Explicitly type the result
    let flat: std.collections.Array[std.num.Int64] = nested.iter().flatMap({ (arr) in arr.iter() }).collect();

    if flat(unchecked: 0) != 1 { return 5 }

    0
}
