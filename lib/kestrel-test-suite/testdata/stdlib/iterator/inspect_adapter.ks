// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test inspect passes elements through unchanged
            let result: std.collections.Array[std.numeric.Int64] = [1, 2, 3].iter().inspect({ (x) in }).collect();
            if result.count != 3 { return 1 }
            if result(unchecked: 0) != 1 { return 2 }
            if result(unchecked: 1) != 2 { return 3 }
            if result(unchecked: 2) != 3 { return 4 }

            // Test inspect in chain - elements still flow through
            let inspected = [1, 2, 3, 4, 5].iter().inspect({ (x) in });
            let filtered: std.collections.Array[std.numeric.Int64] = inspected.filter(matching: { (x) in x > 2 }).collect();
            if filtered.count != 3 { return 5 }
            if filtered(unchecked: 0) != 3 { return 6 }

            // Test inspect on empty iterator
            let empty = std.collections.Array[std.numeric.Int64]();
            let emptyResult = empty.iter().inspect({ (x) in }).collect();
            if emptyResult.count != 0 { return 7 }

            0
        }
