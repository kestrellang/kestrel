// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let r = std.core.Range[std.numeric.Int64](2, 8);

            // contains - value in range
            if r.contains(2) == false { return 1 }
            if r.contains(5) == false { return 2 }
            if r.contains(7) == false { return 3 }

            // contains - end is exclusive
            if r.contains(8) { return 4 }

            // contains - value below range
            if r.contains(1) { return 5 }

            // contains - value above range
            if r.contains(9) { return 6 }

            // isEmpty - non-empty range
            if r.isEmpty { return 7 }

            // isEmpty - empty range (start >= end)
            let emptyRange = std.core.Range[std.numeric.Int64](5, 5);
            if emptyRange.isEmpty == false { return 8 }

            let reverseRange = std.core.Range[std.numeric.Int64](8, 2);
            if reverseRange.isEmpty == false { return 9 }

            // equals
            let r2 = std.core.Range[std.numeric.Int64](2, 8);
            if r.isEqual(to: r2) == false { return 10 }

            let r3 = std.core.Range[std.numeric.Int64](2, 9);
            if r.isEqual(to: r3) { return 11 }

            let r4 = std.core.Range[std.numeric.Int64](3, 8);
            if r.isEqual(to: r4) { return 12 }

            0
        }
