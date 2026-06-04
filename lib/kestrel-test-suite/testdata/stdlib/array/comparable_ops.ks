// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // sort() - in-place ascending
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(3); arr.append(1); arr.append(4); arr.append(1);
            arr.append(5); arr.append(9); arr.append(2); arr.append(6);
            arr.sort();
            if arr(0) != 1 { return 1 }
            if arr(1) != 1 { return 2 }
            if arr(2) != 2 { return 3 }
            if arr(3) != 3 { return 4 }
            if arr(4) != 4 { return 5 }
            if arr(5) != 5 { return 6 }
            if arr(6) != 6 { return 7 }
            if arr(7) != 9 { return 8 }

            // sorted() - returns new sorted array
            var unsorted = std.collections.Array[std.numeric.Int64]();
            unsorted.append(5); unsorted.append(3); unsorted.append(1);
            unsorted.append(4); unsorted.append(2);
            let s = unsorted.sorted();
            if s(0) != 1 { return 9 }
            if s(1) != 2 { return 10 }
            if s(2) != 3 { return 11 }
            if s(3) != 4 { return 12 }
            if s(4) != 5 { return 13 }
            // original unchanged
            if unsorted(0) != 5 { return 14 }

            // min()
            let minVal = unsorted.min();
            if minVal.isNone() { return 15 }
            if minVal.unwrap() != 1 { return 16 }

            // min() on empty
            let empty = std.collections.Array[std.numeric.Int64]();
            let minEmpty = empty.min();
            if minEmpty.isSome() { return 17 }

            // max()
            let maxVal = unsorted.max();
            if maxVal.isNone() { return 18 }
            if maxVal.unwrap() != 5 { return 19 }

            // max() on empty
            let maxEmpty = empty.max();
            if maxEmpty.isSome() { return 20 }

            // isSorted()
            if s.isSorted() == false { return 21 }
            if unsorted.isSorted() { return 22 }
            // empty is sorted
            if empty.isSorted() == false { return 23 }
            // single element is sorted
            var single = std.collections.Array[std.numeric.Int64]();
            single.append(42);
            if single.isSorted() == false { return 24 }
            // equal elements are sorted
            var eq = std.collections.Array[std.numeric.Int64]();
            eq.append(3); eq.append(3); eq.append(3);
            if eq.isSorted() == false { return 25 }

            // binarySearch(element:) - positional single-name param
            var sorted = std.collections.Array[std.numeric.Int64]();
            sorted.append(1); sorted.append(2); sorted.append(3);
            sorted.append(4); sorted.append(5);
            let bs = sorted.binarySearch(3);
            if bs.isNone() { return 26 }
            if bs.unwrap() != 2 { return 27 }

            // binarySearch - not found
            let bsNone = sorted.binarySearch(6);
            if bsNone.isSome() { return 28 }

            // binarySearch - first element
            let bsFirst = sorted.binarySearch(1);
            if bsFirst.isNone() { return 29 }
            if bsFirst.unwrap() != 0 { return 30 }

            // binarySearch - last element
            let bsLast = sorted.binarySearch(5);
            if bsLast.isNone() { return 31 }
            if bsLast.unwrap() != 4 { return 32 }

            0
        }
