// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test reserveCapacity
            var arr = std.collections.Array[std.num.Int64]();
            arr.reserveCapacity(100);
            if arr.capacity < 100 { return 1 }
            if arr.count != 0 { return 2 }

            // Adding elements should not reallocate while under capacity
            arr.append(1);
            arr.append(2);
            arr.append(3);
            if arr.count != 3 { return 3 }
            if arr.capacity < 100 { return 4 }

            // Test shrinkToFit
            arr.shrinkToFit();
            if arr.count != 3 { return 5 }
            if arr.capacity != 3 { return 6 }
            if arr(unchecked: 0) != 1 { return 7 }
            if arr(unchecked: 1) != 2 { return 8 }
            if arr(unchecked: 2) != 3 { return 9 }

            // Test shrinkToFit on empty array with capacity
            var emptyWithCap = std.collections.Array[std.num.Int64](capacity: 50);
            if emptyWithCap.capacity < 50 { return 10 }
            emptyWithCap.shrinkToFit();
            if emptyWithCap.count != 0 { return 11 }

            // Test capacity property via init(capacity:)
            let preallocated = std.collections.Array[std.num.Int64](capacity: 16);
            if preallocated.capacity < 16 { return 12 }
            if preallocated.count != 0 { return 13 }

            // Test that capacity grows after appending beyond initial
            var growing = std.collections.Array[std.num.Int64](capacity: 2);
            growing.append(1);
            growing.append(2);
            growing.append(3);
            if growing.count != 3 { return 14 }
            if growing.capacity < 3 { return 15 }

            0
        }
