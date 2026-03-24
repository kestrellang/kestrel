// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // remove(element:) - removes first occurrence, returns true
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(2); arr.append(4);
            let removed = arr.remove(2);
            if removed == false { return 1 }
            if arr.count != 4 { return 2 }
            // First 2 removed, second 2 remains
            if arr(0) != 1 { return 3 }
            if arr(1) != 3 { return 4 }
            if arr(2) != 2 { return 5 }
            if arr(3) != 4 { return 6 }

            // remove(element:) - element not found, returns false
            let notRemoved = arr.remove(99);
            if notRemoved { return 7 }
            if arr.count != 4 { return 8 }

            // remove(element:) on empty array
            var emptyArr = std.collections.Array[std.num.Int64]();
            let emptyRemoved = emptyArr.remove(1);
            if emptyRemoved { return 9 }

            // removeAll(element:) - removes all occurrences
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(1); arr2.append(2); arr2.append(3); arr2.append(2); arr2.append(4); arr2.append(2);
            arr2.removeAll(2);
            if arr2.count != 3 { return 10 }
            if arr2(0) != 1 { return 11 }
            if arr2(1) != 3 { return 12 }
            if arr2(2) != 4 { return 13 }

            // removeAll(element:) - element not present
            var arr3 = std.collections.Array[std.num.Int64]();
            arr3.append(1); arr3.append(2); arr3.append(3);
            arr3.removeAll(99);
            if arr3.count != 3 { return 14 }

            // removeAll(element:) - remove all elements (all same)
            var arr4 = std.collections.Array[std.num.Int64]();
            arr4.append(5); arr4.append(5); arr4.append(5);
            arr4.removeAll(5);
            if arr4.count != 0 { return 15 }

            0
        }
