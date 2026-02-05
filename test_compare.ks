module Test

func main() -> lang.i64 {
    // Test: compactMap with comparison
    var arrOpt = std.collections.Array[std.result.Optional[std.num.Int64]]();
    arrOpt.append(.Some(1));

    let compacted = arrOpt.iter().compactMap().collect();

    // Just try the comparison without the if
    let elem = compacted(unchecked: 0);
    let result = elem != 1;

    0
}
