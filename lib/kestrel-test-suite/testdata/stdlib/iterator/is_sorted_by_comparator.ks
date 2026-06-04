// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // isSorted(by:) with a custom comparator
            // Check descending order: a >= b means "a comes before b"
            if [5, 4, 3, 2, 1].iter().isSorted(by: { (a, b) in a >= b }) == false { return 1 }

            // Ascending is not sorted in descending order
            if [1, 2, 3, 4, 5].iter().isSorted(by: { (a, b) in a >= b }) { return 2 }

            // Check sorted by absolute value
            if [-1, 2, -3, 4].iter().isSorted(by: { (a, b) in
                let absA = if a < 0 { 0 - a } else { a };
                let absB = if b < 0 { 0 - b } else { b };
                absA <= absB
            }) == false { return 3 }

            // Not sorted by absolute value
            if [3, -1, 2].iter().isSorted(by: { (a, b) in
                let absA = if a < 0 { 0 - a } else { a };
                let absB = if b < 0 { 0 - b } else { b };
                absA <= absB
            }) { return 4 }

            // Empty iterator is sorted by any comparator
            let empty = std.collections.Array[std.numeric.Int64]();
            if empty.iter().isSorted(by: { (a, b) in false }) == false { return 5 }

            // Single element is sorted by any comparator
            if [42].iter().isSorted(by: { (a, b) in false }) == false { return 6 }

            // Equal elements - ascending comparator
            if [3, 3, 3].iter().isSorted(by: { (a, b) in a <= b }) == false { return 7 }

            0
        }
