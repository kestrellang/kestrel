// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test init(capacity:)
            var arr = std.collections.Array[std.num.Int64](capacity: 10);
            if arr.count != 0 { return 1 }
            if arr.capacity < 10 { return 2 }
            arr.append(42);
            if arr.count != 1 { return 3 }
            if arr(unchecked: 0) != 42 { return 4 }

            // NOTE: init(repeating:count:) requires T: Cloneable, but Int64 does not
            // implement Cloneable, causing monomorphization failure. See init_repeating test.

            // Test init(from:) with a range
            let fromRange = std.collections.Array[std.num.Int64](from: std.core.Range[std.num.Int64](0, 5));
            if fromRange.count != 5 { return 5 }
            if fromRange(unchecked: 0) != 0 { return 6 }
            if fromRange(unchecked: 4) != 4 { return 7 }

            // NOTE: init(count:generator:) cannot be tested because its positional
            // signature Array(Int64, (Int64) -> T) collides with the internal
            // array literal init Array(lang.ptr[T], lang.i64). See init_count_generator test.

            0
        }
