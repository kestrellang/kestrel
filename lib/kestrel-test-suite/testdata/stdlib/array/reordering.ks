// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test swap
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);
            arr.swap(at: 0, with: 2);
            if arr(unchecked: 0) != 30 { return 1 }
            if arr(unchecked: 1) != 20 { return 2 }
            if arr(unchecked: 2) != 10 { return 3 }

            // Test swap same index (no-op)
            arr.swap(at: 1, with: 1);
            if arr(unchecked: 1) != 20 { return 4 }

            // Test reverse
            arr.reverse();
            if arr(unchecked: 0) != 10 { return 5 }
            if arr(unchecked: 1) != 20 { return 6 }
            if arr(unchecked: 2) != 30 { return 7 }

            // Test reversed (returns new array, original unchanged)
            let rev = arr.reversed();
            if rev(unchecked: 0) != 30 { return 8 }
            if rev(unchecked: 1) != 20 { return 9 }
            if rev(unchecked: 2) != 10 { return 10 }
            // Original should be unchanged
            if arr(unchecked: 0) != 10 { return 11 }

            // Test rotate left by 2
            var rotArr = std.collections.Array[std.numeric.Int64]();
            rotArr.append(1);
            rotArr.append(2);
            rotArr.append(3);
            rotArr.append(4);
            rotArr.append(5);
            rotArr.rotate(by: 2);
            // [1, 2, 3, 4, 5] rotated left by 2 = [3, 4, 5, 1, 2]
            if rotArr(unchecked: 0) != 3 { return 12 }
            if rotArr(unchecked: 1) != 4 { return 13 }
            if rotArr(unchecked: 2) != 5 { return 14 }
            if rotArr(unchecked: 3) != 1 { return 15 }
            if rotArr(unchecked: 4) != 2 { return 16 }

            // Test replaceSubrange
            var repArr = std.collections.Array[std.numeric.Int64]();
            repArr.append(1);
            repArr.append(2);
            repArr.append(3);
            repArr.append(4);
            repArr.append(5);
            var replacement = std.collections.Array[std.numeric.Int64]();
            replacement.append(20);
            replacement.append(30);
            // Replace range 1..<4 ([2,3,4]) with [20,30]
            repArr.replaceSubrange(std.core.Range[std.numeric.Int64](1, 4), with: replacement);
            // Result should be [1, 20, 30, 5]
            if repArr.count != 4 { return 17 }
            if repArr(unchecked: 0) != 1 { return 18 }
            if repArr(unchecked: 1) != 20 { return 19 }
            if repArr(unchecked: 2) != 30 { return 20 }
            if repArr(unchecked: 3) != 5 { return 21 }

            0
        }
