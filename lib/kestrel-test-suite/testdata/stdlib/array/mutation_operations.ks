// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test append(contentsOf:)
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1);
            arr.append(2);
            var other = std.collections.Array[std.numeric.Int64]();
            other.append(3);
            other.append(4);
            arr.append(contentsOf: other);
            if arr.count != 4 { return 1 }
            if arr(unchecked: 2) != 3 { return 2 }
            if arr(unchecked: 3) != 4 { return 3 }

            // Test append(contentsOf:) with empty array
            let empty = std.collections.Array[std.numeric.Int64]();
            arr.append(contentsOf: empty);
            if arr.count != 4 { return 4 }

            // Test insert(element:at:) at beginning
            arr.insert(0, at: 0);
            if arr.count != 5 { return 5 }
            if arr(unchecked: 0) != 0 { return 6 }
            if arr(unchecked: 1) != 1 { return 7 }

            // Test insert(element:at:) in middle
            arr.insert(99, at: 3);
            if arr.count != 6 { return 8 }
            if arr(unchecked: 3) != 99 { return 9 }
            if arr(unchecked: 4) != 3 { return 10 }

            // Test insert(element:at:) at end (append)
            arr.insert(100, at: 6);
            if arr.count != 7 { return 11 }
            if arr(unchecked: 6) != 100 { return 12 }

            // Test popFirst()
            let first = arr.popFirst();
            if first.isNone() { return 13 }
            if first.unwrap() != 0 { return 14 }
            if arr.count != 6 { return 15 }
            if arr(unchecked: 0) != 1 { return 16 }

            // Test popFirst() on empty
            var emptyArr = std.collections.Array[std.numeric.Int64]();
            let emptyFirst = emptyArr.popFirst();
            if emptyFirst.isSome() { return 17 }

            // Test remove(at:)
            // arr is now [1, 2, 99, 3, 4, 100]
            let removed = arr.remove(at: 2);
            if removed != 99 { return 18 }
            if arr.count != 5 { return 19 }
            if arr(unchecked: 2) != 3 { return 20 }

            // Test removeSubrange
            // arr is now [1, 2, 3, 4, 100]
            arr.removeSubrange(std.core.Range[std.numeric.Int64](1, 3));
            if arr.count != 3 { return 21 }
            if arr(unchecked: 0) != 1 { return 22 }
            if arr(unchecked: 1) != 4 { return 23 }
            if arr(unchecked: 2) != 100 { return 24 }

            // Test clear()
            arr.clear();
            if arr.count != 0 { return 25 }
            if arr.isEmpty == false { return 26 }

            0
        }
