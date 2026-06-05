// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // capacity property
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64](capacity: 32);
            if dict.capacity < 32 { return 1 }
            if dict.count != 0 { return 2 }

            // reserveCapacity
            var dict2 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            dict2.reserveCapacity(64);
            if dict2.capacity < 64 { return 3 }
            if dict2.count != 0 { return 4 }

            // Insert elements and verify they work after reserveCapacity
            let _ = dict2.insert(1, 10);
            let _ = dict2.insert(2, 20);
            let _ = dict2.insert(3, 30);
            if dict2.count != 3 { return 5 }
            if dict2(1).unwrap() != 10 { return 6 }
            if dict2(2).unwrap() != 20 { return 7 }
            if dict2(3).unwrap() != 30 { return 8 }

            // shrinkToFit - reduces capacity
            var dict3 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64](capacity: 256);
            let _ = dict3.insert(1, 10);
            let _ = dict3.insert(2, 20);
            let capBefore = dict3.capacity;
            dict3.shrinkToFit();
            let capAfter = dict3.capacity;
            if capAfter >= capBefore { return 9 }
            // Data should be preserved
            if dict3.count != 2 { return 10 }
            if dict3(1).unwrap() != 10 { return 11 }
            if dict3(2).unwrap() != 20 { return 12 }

            // shrinkToFit on empty dictionary
            var dict4 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64](capacity: 64);
            dict4.shrinkToFit();
            if dict4.count != 0 { return 13 }

            // reserveCapacity when already sufficient - no-op
            var dict5 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64](capacity: 128);
            let capPrev = dict5.capacity;
            dict5.reserveCapacity(16);
            if dict5.capacity != capPrev { return 14 }

            0
        }
