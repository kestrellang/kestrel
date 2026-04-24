// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test isSorted - ascending
            if [1, 2, 3, 4, 5].iter().isSorted() == false { return 1 }

            // Test isSorted - not sorted
            if [1, 3, 2, 4, 5].iter().isSorted() { return 2 }

            // Test isSorted - equal elements OK
            if [1, 1, 2, 2, 3].iter().isSorted() == false { return 3 }

            // Test isSorted - empty
            let empty = std.collections.Array[std.num.Int64]();
            if empty.iter().isSorted() == false { return 4 }

            // Test isSorted - single element
            if [42].iter().isSorted() == false { return 5 }

            // Test isSortedDescending - descending
            if [5, 4, 3, 2, 1].iter().isSortedDescending() == false { return 6 }

            // Test isSortedDescending - not descending
            if [5, 3, 4, 2, 1].iter().isSortedDescending() { return 7 }

            // Test isSortedDescending - equal elements OK
            if [3, 3, 2, 2, 1].iter().isSortedDescending() == false { return 8 }

            // Test isSortedDescending - empty
            if empty.iter().isSortedDescending() == false { return 9 }

            // Test isSortedDescending - single element
            if [42].iter().isSortedDescending() == false { return 10 }

            // Test isSortedDescending - ascending is not descending (unless single/empty)
            if [1, 2, 3].iter().isSortedDescending() { return 11 }

            0
        }
