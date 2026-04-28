// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Create slice from array's asSlice()
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);
            arr.append(40);
            arr.append(50);

            let slice = arr.asSlice();

            // Test count
            if slice.count != 5 { return 1 }

            // Test isEmpty
            if slice.isEmpty { return 2 }

            // Test isEmpty on empty slice
            let emptyArr = std.collections.Array[std.numeric.Int64]();
            let emptySlice = emptyArr.asSlice();
            if emptySlice.isEmpty == false { return 3 }
            if emptySlice.count != 0 { return 4 }

            // Test subscript(unchecked:) get
            if slice(unchecked: 0) != 10 { return 5 }
            if slice(unchecked: 1) != 20 { return 6 }
            if slice(unchecked: 2) != 30 { return 7 }
            if slice(unchecked: 3) != 40 { return 8 }
            if slice(unchecked: 4) != 50 { return 9 }

            // Test subscript(checked:) - valid index
            let safe1 = slice(checked: 2);
            if safe1.isNone() { return 10 }
            if safe1.unwrap() != 30 { return 11 }

            // Test subscript(checked:) - out of bounds
            let safeOob = slice(checked: 10);
            if safeOob.isSome() { return 12 }

            // Test subscript(checked:) - negative index
            let safeNeg = slice(checked: -1);
            if safeNeg.isSome() { return 13 }

            // Test first()
            let f = slice.first();
            if f.isNone() { return 14 }
            if f.unwrap() != 10 { return 15 }

            // Test last()
            let l = slice.last();
            if l.isNone() { return 16 }
            if l.unwrap() != 50 { return 17 }

            // Test first() and last() on empty slice
            if emptySlice.first().isSome() { return 18 }
            if emptySlice.last().isSome() { return 19 }

            // Test subscript(checked: Range) - valid sub-slice
            let sub = slice(checked: std.core.Range[std.numeric.Int64](1, 4));
            if sub.isNone() { return 20 }
            let subSlice = sub.unwrap();
            if subSlice.count != 3 { return 21 }
            if subSlice(unchecked: 0) != 20 { return 22 }
            if subSlice(unchecked: 1) != 30 { return 23 }
            if subSlice(unchecked: 2) != 40 { return 24 }

            // Test subscript(checked: Range) - empty sub-slice
            let emptySub = slice(checked: std.core.Range[std.numeric.Int64](2, 2));
            if emptySub.isNone() { return 25 }
            if emptySub.unwrap().count != 0 { return 26 }

            // Test subscript(checked: Range) - full range
            let fullSub = slice(checked: std.core.Range[std.numeric.Int64](0, 5));
            if fullSub.isNone() { return 27 }
            if fullSub.unwrap().count != 5 { return 28 }

            // Test subscript(checked: Range) - invalid range returns None
            let invalidSub = slice(checked: std.core.Range[std.numeric.Int64](3, 1));
            if invalidSub.isSome() { return 29 }

            // Test subscript(checked: Range) - out of bounds returns None
            let oobSub = slice(checked: std.core.Range[std.numeric.Int64](0, 10));
            if oobSub.isSome() { return 30 }

            // Test iter()
            var iter = slice.iter();
            var sum: std.numeric.Int64 = 0;
            var done: std.core.Bool = false;
            while done == false {
                let next = iter.next();
                if next.isSome() {
                    sum = sum + next.unwrap()
                } else {
                    done = true
                }
            }
            if sum != 150 { return 31 }

            // Test pointer property
            let ptr = slice.pointer;
            if ptr.isNull { return 32 }
            if ptr.read() != 10 { return 33 }

            0
        }
