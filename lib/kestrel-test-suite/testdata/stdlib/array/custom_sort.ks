// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // sort(by:) - descending order
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(5); arr.append(3); arr.append(2); arr.append(4);
            arr.sort(by: { (a, b) in a > b });
            if arr(0) != 5 { return 1 }
            if arr(1) != 4 { return 2 }
            if arr(2) != 3 { return 3 }
            if arr(3) != 2 { return 4 }
            if arr(4) != 1 { return 5 }

            // sorted(by:) - returns new array
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(3); arr2.append(1); arr2.append(4); arr2.append(1); arr2.append(5);
            let desc = arr2.sorted(by: { (a, b) in a > b });
            if desc(0) != 5 { return 6 }
            if desc(1) != 4 { return 7 }
            if desc(2) != 3 { return 8 }
            if desc(3) != 1 { return 9 }
            if desc(4) != 1 { return 10 }
            // original unchanged
            if arr2(0) != 3 { return 11 }

            // sort(byKey:) - sort by absolute value (using negative values)
            var arr3 = std.collections.Array[std.num.Int64]();
            arr3.append(3); arr3.append(-1); arr3.append(4); arr3.append(-5); arr3.append(2);
            arr3.sort(byKey: { (x) in if x < 0 { 0 - x } else { x } });
            if arr3(0) != -1 { return 12 }
            if arr3(1) != 2 { return 13 }
            if arr3(2) != 3 { return 14 }
            if arr3(3) != 4 { return 15 }
            if arr3(4) != -5 { return 16 }

            // sorted(byKey:) - returns new array
            var arr4 = std.collections.Array[std.num.Int64]();
            arr4.append(-3); arr4.append(1); arr4.append(-2);
            let byAbs = arr4.sorted(byKey: { (x) in if x < 0 { 0 - x } else { x } });
            if byAbs(0) != 1 { return 17 }
            if byAbs(1) != -2 { return 18 }
            if byAbs(2) != -3 { return 19 }
            // original unchanged
            if arr4(0) != -3 { return 20 }

            0
        }
