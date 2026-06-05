// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test capacity property on init(capacity:)
            var s = std.collections.Set[std.numeric.Int64](capacity: 100);
            if s.capacity < 100 { return 1 }
            if s.count != 0 { return 2 }

            // Test reserveCapacity
            var s2 = std.collections.Set[std.numeric.Int64]();
            s2.reserveCapacity(50);
            if s2.capacity < 50 { return 3 }

            // reserveCapacity doesn't shrink
            let capBefore = s2.capacity;
            s2.reserveCapacity(10);
            if s2.capacity < capBefore { return 4 }

            // Test shrinkToFit
            var s3 = std.collections.Set[std.numeric.Int64](capacity: 100);
            let _ = s3.insert(1);
            let _ = s3.insert(2);
            let capBeforeShrink = s3.capacity;
            s3.shrinkToFit();
            if s3.capacity > capBeforeShrink { return 5 }
            // Elements should still be there after shrink
            if s3.count != 2 { return 6 }
            if s3.contains(1) == false { return 7 }
            if s3.contains(2) == false { return 8 }

            0
        }
