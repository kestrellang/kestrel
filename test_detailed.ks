module Test

func main() -> lang.i64 {
    var arrOpt = std.collections.Array[std.result.Optional[std.num.Int64]]();
    arrOpt.append(.Some(1));

    // Test 1: Can we call compactMap and get an iterator?
    var compactIter = arrOpt.iter().compactMap();

    // Test 2: Can we call next() on it?
    let first = compactIter.next();

    // Test 3: Can we unwrap and compare?
    if let .Some(val) = first {
        if val != 1 { return 1 }
    }

    0
}
