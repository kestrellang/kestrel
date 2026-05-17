// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- fuse() ----
            let fused: std.collections.Array[std.numeric.Int64] = [1, 2, 3].iter().fuse().collect();
            if fused.count != 3 { return 1 }
            if fused(unchecked: 0) != 1 { return 2 }
            if fused(unchecked: 2) != 3 { return 3 }

            // ---- cycle() + take() ----
            let cycled: std.collections.Array[std.numeric.Int64] = [1, 2, 3].iter().cycle().take(7).collect();
            if cycled.count != 7 { return 4 }
            if cycled(unchecked: 0) != 1 { return 5 }
            if cycled(unchecked: 3) != 1 { return 6 }
            if cycled(unchecked: 6) != 1 { return 7 }

            0
        }
