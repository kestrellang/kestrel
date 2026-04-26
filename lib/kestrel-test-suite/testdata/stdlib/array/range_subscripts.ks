// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10); arr.append(20); arr.append(30); arr.append(40); arr.append(50);

            // subscript(_:) — Range arg, basic slice
            let slice = arr(std.core.Range[std.num.Int64](1, 4));
            if slice.count != 3 { return 1 }
            if slice(unchecked: 0) != 20 { return 2 }
            if slice(unchecked: 1) != 30 { return 3 }
            if slice(unchecked: 2) != 40 { return 4 }

            // subscript(_:) — Range arg, full range
            let full = arr(std.core.Range[std.num.Int64](0, 5));
            if full.count != 5 { return 5 }
            if full(unchecked: 0) != 10 { return 6 }
            if full(unchecked: 4) != 50 { return 7 }

            // subscript(_:) — Range arg, empty range
            let emptySlice = arr(std.core.Range[std.num.Int64](2, 2));
            if emptySlice.count != 0 { return 8 }

            // subscript(checked:) — Range arg, valid
            let checked = arr(checked: std.core.Range[std.num.Int64](0, 3));
            if checked.isNone() { return 9 }
            let checkedSlice = checked.unwrap();
            if checkedSlice.count != 3 { return 10 }
            if checkedSlice(unchecked: 0) != 10 { return 11 }
            if checkedSlice(unchecked: 2) != 30 { return 12 }

            // subscript(checked:) — Range arg, end out of bounds
            let oob = arr(checked: std.core.Range[std.num.Int64](0, 10));
            if oob.isSome() { return 13 }

            // subscript(checked:) — Range arg, negative start
            let negStart = arr(checked: std.core.Range[std.num.Int64](-1, 3));
            if negStart.isSome() { return 14 }

            // subscript(unchecked:) — Range arg
            let unchecked = arr(unchecked: std.core.Range[std.num.Int64](2, 4));
            if unchecked.count != 2 { return 15 }
            if unchecked(unchecked: 0) != 30 { return 16 }
            if unchecked(unchecked: 1) != 40 { return 17 }

            // subscript(clamped:) - range fully within bounds
            let clamped = arr(clamped: std.core.Range[std.num.Int64](1, 3));
            if clamped.count != 2 { return 18 }
            if clamped(unchecked: 0) != 20 { return 19 }
            if clamped(unchecked: 1) != 30 { return 20 }

            // subscript(clamped:) - out of bounds range clamped
            let clampedWide = arr(clamped: std.core.Range[std.num.Int64](-5, 100));
            if clampedWide.count != 5 { return 21 }
            if clampedWide(unchecked: 0) != 10 { return 22 }
            if clampedWide(unchecked: 4) != 50 { return 23 }

            // subscript(clamped:) - both indices past end
            let clampedPast = arr(clamped: std.core.Range[std.num.Int64](10, 20));
            if clampedPast.count != 0 { return 24 }

            // subscript(clamped:) - negative range clamped to start
            let clampedNeg = arr(clamped: std.core.Range[std.num.Int64](-5, 1));
            if clampedNeg.count != 1 { return 25 }
            if clampedNeg(unchecked: 0) != 10 { return 26 }

            0
        }
