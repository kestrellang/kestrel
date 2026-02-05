module Test

func main() -> lang.i64 {
    // Test 1: Simple array iteration - does this work?
    let arr: std.collections.Array[std.num.Int64] = [1, 2, 3];
    let first = arr.iter().next().unwrap();
    if first != 1 { return 1 }

    // Test 2: With collect - does this work?
    let collected = arr.iter().collect();
    if collected(unchecked: 0) != 1 { return 2 }

    // Test 3: With map - where does it break?
    let mapped = arr.iter().map({ (x) in x * 2 }).collect();
    if mapped(unchecked: 0) != 2 { return 3 }

    // Test 4: With filter - where does it break?
    let filtered = arr.iter().filter({ (x) in x > 1 }).collect();
    if filtered(unchecked: 0) != 2 { return 4 }

    // Test 5: compactMap - the failing case
    var arrOpt = std.collections.Array[std.result.Optional[std.num.Int64]]();
    arrOpt.append(.Some(1));
    arrOpt.append(.None);
    arrOpt.append(.Some(3));

    let compacted = arrOpt.iter().compactMap().collect();
    if compacted.count != 2 { return 5 }
    if compacted(unchecked: 0) != 1 { return 6 }
    if compacted(unchecked: 1) != 3 { return 7 }

    0
}
