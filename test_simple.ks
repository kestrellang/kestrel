module Test

func main() -> lang.i64 {
    // Test: Just compactMap without comparison
    var arrOpt = std.collections.Array[std.result.Optional[std.num.Int64]]();
    arrOpt.append(.Some(1));

    let compacted = arrOpt.iter().compactMap().collect();

    // Try to access the element
    let elem = compacted(unchecked: 0);

    // Try to return it directly - this should show what type it thinks it is
    elem
}
