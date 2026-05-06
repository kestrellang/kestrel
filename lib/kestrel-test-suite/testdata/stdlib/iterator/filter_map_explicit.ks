// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test filterMap with explicit transform returning Optional
            let result: std.collections.Array[std.numeric.Int64] = [1, 2, 3, 4, 5].iter().filterMap(as: { (x) in if x % 2 == 0 { .Some(x * 10) } else { .None } }).collect();
            if result.count != 2 { return 1 }
            if result(unchecked: 0) != 20 { return 2 }
            if result(unchecked: 1) != 40 { return 3 }

            // filterMap where all are None
            let allNone: std.collections.Array[std.numeric.Int64] = [1, 2, 3].iter().filterMap(as: { (x) in .None }).collect();
            if allNone.count != 0 { return 4 }

            // filterMap where all are Some
            let allSome: std.collections.Array[std.numeric.Int64] = [1, 2, 3].iter().filterMap(as: { (x) in .Some(x) }).collect();
            if allSome.count != 3 { return 5 }

            0
        }
