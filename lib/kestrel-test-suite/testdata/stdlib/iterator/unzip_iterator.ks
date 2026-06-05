// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test unzip on iterator of tuples
            var pairs = std.collections.Array[(std.numeric.Int64, std.numeric.Int64)]();
            pairs.append((1, 10));
            pairs.append((2, 20));
            pairs.append((3, 30));

            let (left, right) = pairs.iter().unzip();
            if left.count != 3 { return 1 }
            if right.count != 3 { return 2 }
            if left(unchecked: 0) != 1 { return 3 }
            if left(unchecked: 1) != 2 { return 4 }
            if left(unchecked: 2) != 3 { return 5 }
            if right(unchecked: 0) != 10 { return 6 }
            if right(unchecked: 1) != 20 { return 7 }
            if right(unchecked: 2) != 30 { return 8 }

            // Unzip empty
            let emptyPairs = std.collections.Array[(std.numeric.Int64, std.numeric.Int64)]();
            let (emptyLeft, emptyRight) = emptyPairs.iter().unzip();
            if emptyLeft.count != 0 { return 9 }
            if emptyRight.count != 0 { return 10 }

            0
        }
