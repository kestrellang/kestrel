// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- intersperse() ----
            let result: std.collections.Array[std.numeric.Int64] = [1, 2, 3].iter().intersperse(0).collect();
            if result.count != 5 { return 1 }
            if result(unchecked: 0) != 1 { return 2 }
            if result(unchecked: 1) != 0 { return 3 }
            if result(unchecked: 2) != 2 { return 4 }
            if result(unchecked: 3) != 0 { return 5 }
            if result(unchecked: 4) != 3 { return 6 }

            // Single element - no separator
            let single: std.collections.Array[std.numeric.Int64] = [42].iter().intersperse(0).collect();
            if single.count != 1 { return 7 }
            if single(unchecked: 0) != 42 { return 8 }

            // Empty - stays empty
            let empty = std.collections.Array[std.numeric.Int64]();
            let emptyResult = empty.iter().intersperse(0).collect();
            if emptyResult.count != 0 { return 9 }

            0
        }
