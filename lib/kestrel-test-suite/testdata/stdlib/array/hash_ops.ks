// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // unique() - returns new array with duplicates removed, preserving first occurrence order
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(1); arr.append(3); arr.append(2); arr.append(4);
            let u = arr.unique();
            if u.count != 4 { return 1 }
            if u(0) != 1 { return 2 }
            if u(1) != 2 { return 3 }
            if u(2) != 3 { return 4 }
            if u(3) != 4 { return 5 }
            // original unchanged
            if arr.count != 6 { return 6 }

            // unique() on array with no duplicates
            var noDups = std.collections.Array[std.num.Int64]();
            noDups.append(1); noDups.append(2); noDups.append(3);
            let noDupsU = noDups.unique();
            if noDupsU.count != 3 { return 7 }

            // unique() on empty
            let empty = std.collections.Array[std.num.Int64]();
            let emptyU = empty.unique();
            if emptyU.count != 0 { return 8 }

            // removeDuplicates() - in place
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(1); arr2.append(2); arr2.append(1); arr2.append(3); arr2.append(2);
            arr2.removeDuplicates();
            if arr2.count != 3 { return 9 }
            if arr2(0) != 1 { return 10 }
            if arr2(1) != 2 { return 11 }
            if arr2(2) != 3 { return 12 }

            // removeDuplicates on all same
            var allSame = std.collections.Array[std.num.Int64]();
            allSame.append(5); allSame.append(5); allSame.append(5); allSame.append(5);
            allSame.removeDuplicates();
            if allSame.count != 1 { return 13 }
            if allSame(0) != 5 { return 14 }

            0
        }
