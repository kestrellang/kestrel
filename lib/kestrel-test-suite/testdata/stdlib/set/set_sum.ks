// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var s = std.collections.Set[std.numeric.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);

            let total = s.sum();
            if total != 6 { return 1 }

            // Empty set sum
            let empty = std.collections.Set[std.numeric.Int64]();
            let emptySum = empty.sum();
            if emptySum != 0 { return 2 }

            // Single element
            var single = std.collections.Set[std.numeric.Int64]();
            let _ = single.insert(42);
            if single.sum() != 42 { return 3 }

            0
        }
