// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);

            // shuffled() returns a new array with same count
            let result = arr.shuffled();
            if result.count != 5 { return 1 }

            // Original unchanged
            if arr(0) != 1 { return 2 }
            if arr(1) != 2 { return 3 }
            if arr(2) != 3 { return 4 }
            if arr(3) != 4 { return 5 }
            if arr(4) != 5 { return 6 }

            // shuffled result contains all original elements
            if result.contains(1) == false { return 7 }
            if result.contains(2) == false { return 8 }
            if result.contains(3) == false { return 9 }
            if result.contains(4) == false { return 10 }
            if result.contains(5) == false { return 11 }

            // shuffle() mutating - same count and same elements
            var arr2 = std.collections.Array[std.numeric.Int64]();
            arr2.append(10); arr2.append(20); arr2.append(30);
            arr2.shuffle();
            if arr2.count != 3 { return 12 }
            if arr2.contains(10) == false { return 13 }
            if arr2.contains(20) == false { return 14 }
            if arr2.contains(30) == false { return 15 }

            // shuffled on empty array
            let empty = std.collections.Array[std.numeric.Int64]();
            let emptyResult = empty.shuffled();
            if emptyResult.count != 0 { return 16 }

            // shuffled on single element array
            var single = std.collections.Array[std.numeric.Int64]();
            single.append(42);
            let singleResult = single.shuffled();
            if singleResult.count != 1 { return 17 }
            if singleResult(0) != 42 { return 18 }

            0
        }
