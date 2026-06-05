// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test partition(by:) - in-place, returns pivot index
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);
            let pivot = arr.partition(by: { (x) in x % 2 == 0 });
            // After partition: even elements come first, pivot is the count of even elements
            if pivot != 2 { return 1 }
            // The first `pivot` elements should all be even
            var i: std.numeric.Int64 = 0;
            while i < pivot {
                if arr(i) % 2 != 0 { return 2 }
                i = i + 1
            }
            // The remaining elements should all be odd
            while i < arr.count {
                if arr(i) % 2 == 0 { return 3 }
                i = i + 1
            }

            // Test partitioned(by:) - returns two arrays, preserves order
            var arr2 = std.collections.Array[std.numeric.Int64]();
            arr2.append(1); arr2.append(2); arr2.append(3); arr2.append(4); arr2.append(5);
            let (evens, odds) = arr2.partitioned(by: { (x) in x % 2 == 0 });
            if evens.count != 2 { return 4 }
            if odds.count != 3 { return 5 }
            if evens(0) != 2 { return 6 }
            if evens(1) != 4 { return 7 }
            if odds(0) != 1 { return 8 }
            if odds(1) != 3 { return 9 }
            if odds(2) != 5 { return 10 }

            // partitioned on empty array
            let empty = std.collections.Array[std.numeric.Int64]();
            let (emptyMatch, emptyNot) = empty.partitioned(by: { (x) in true });
            if emptyMatch.count != 0 { return 11 }
            if emptyNot.count != 0 { return 12 }

            0
        }
