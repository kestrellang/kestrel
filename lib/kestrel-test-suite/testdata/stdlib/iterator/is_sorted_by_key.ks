// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // isSortedBy(key:) checks if elements are sorted by extracted key ascending

            // Sorted by absolute value
            if [-1, 2, -3, 4].iter().isSortedBy({ (x) in if x < 0 { 0 - x } else { x } }) == false { return 1 }

            // Not sorted by absolute value
            if [3, -1, 2].iter().isSortedBy({ (x) in if x < 0 { 0 - x } else { x } }) { return 2 }

            // Sorted by negation (effectively descending by value)
            if [5, 4, 3, 2, 1].iter().isSortedBy({ (x) in 0 - x }) == false { return 3 }

            // Not sorted by negation
            if [1, 2, 3].iter().isSortedBy({ (x) in 0 - x }) { return 4 }

            // Empty - always sorted
            let empty = std.collections.Array[std.numeric.Int64]();
            if empty.iter().isSortedBy({ (x) in x }) == false { return 5 }

            // Single element - always sorted
            if [42].iter().isSortedBy({ (x) in x }) == false { return 6 }

            // Identity key - same as isSorted()
            if [1, 2, 3, 4, 5].iter().isSortedBy({ (x) in x }) == false { return 7 }
            if [1, 3, 2].iter().isSortedBy({ (x) in x }) { return 8 }

            0
        }
