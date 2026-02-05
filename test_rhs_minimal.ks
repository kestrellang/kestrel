module Test

func main() -> lang.i64 {
    var nested = std.collections.Array[std.collections.Array[std.num.Int64]]();
    var inner1 = std.collections.Array[std.num.Int64]();
    inner1.append(1);
    nested.append(inner1);

    let flat = nested.iter().flatMap({ (arr) in arr.iter() }).collect();

    // This should fail with the Rhs issue
    if flat(unchecked: 0) != 1 { return 5 }

    0
}
