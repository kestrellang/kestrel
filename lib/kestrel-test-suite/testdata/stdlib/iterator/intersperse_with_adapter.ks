// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // intersperseWith: lazy separator via closure
            let result: std.collections.Array[std.numeric.Int64] = [1, 2, 3].iter().intersperseWith(with: { () in 0 }).collect();
            if result.count != 5 { return 1 }
            if result(unchecked: 0) != 1 { return 2 }
            if result(unchecked: 1) != 0 { return 3 }
            if result(unchecked: 2) != 2 { return 4 }
            if result(unchecked: 3) != 0 { return 5 }
            if result(unchecked: 4) != 3 { return 6 }

            // Single element - no separator generated
            let single: std.collections.Array[std.numeric.Int64] = [42].iter().intersperseWith(with: { () in 0 }).collect();
            if single.count != 1 { return 7 }
            if single(unchecked: 0) != 42 { return 8 }

            // Empty iterator - stays empty
            let empty = std.collections.Array[std.numeric.Int64]();
            let emptyResult = empty.iter().intersperseWith(with: { () in 99 }).collect();
            if emptyResult.count != 0 { return 9 }

            // intersperseWith with varying separator (counter-based)
            // Note: cannot use mutable closure captures, so use a constant separator
            let result2: std.collections.Array[std.numeric.Int64] = [10, 20, 30].iter().intersperseWith(with: { () in -1 }).collect();
            if result2.count != 5 { return 10 }
            if result2(unchecked: 0) != 10 { return 11 }
            if result2(unchecked: 1) != -1 { return 12 }
            if result2(unchecked: 2) != 20 { return 13 }
            if result2(unchecked: 3) != -1 { return 14 }
            if result2(unchecked: 4) != 30 { return 15 }

            0
        }
