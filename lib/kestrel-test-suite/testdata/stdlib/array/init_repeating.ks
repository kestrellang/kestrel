// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let arr = std.collections.Array[std.num.Int64](repeating: 7, count: 5);
            if arr.count != 5 { return 1 }
            if arr(0) != 7 { return 2 }
            if arr(1) != 7 { return 3 }
            if arr(4) != 7 { return 4 }

            // repeating with count 0
            let empty = std.collections.Array[std.num.Int64](repeating: 42, count: 0);
            if empty.count != 0 { return 5 }

            // repeating with count 1
            let single = std.collections.Array[std.num.Int64](repeating: 99, count: 1);
            if single.count != 1 { return 6 }
            if single(0) != 99 { return 7 }

            0
        }
