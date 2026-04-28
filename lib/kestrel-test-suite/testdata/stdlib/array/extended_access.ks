// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);

            // Test subscript(index:) get
            if arr(0) != 10 { return 1 }
            if arr(1) != 20 { return 2 }
            if arr(2) != 30 { return 3 }

            // Test subscript set via arr(i) = v
            arr(1) = 25;
            if arr(1) != 25 { return 4 }
            // Also exercise unchecked subscript for equivalence
            arr(unchecked: 1) = 25;

            // Test subscript(wrapped:) with negative index
            let wrapLast = arr(wrapped: -1);
            if wrapLast.isNone() { return 5 }
            if wrapLast.unwrap() != 30 { return 6 }

            // Test subscript(wrapped:) with -2
            let wrapSecond = arr(wrapped: -2);
            if wrapSecond.isNone() { return 7 }
            if wrapSecond.unwrap() != 25 { return 8 }

            // Test subscript(wrapped:) with overflow
            let wrapOver = arr(wrapped: 3);
            if wrapOver.isNone() { return 9 }
            if wrapOver.unwrap() != 10 { return 10 }

            // Test subscript(wrapped:) on empty array
            let emptyArr = std.collections.Array[std.numeric.Int64]();
            let wrapEmpty = emptyArr(wrapped: 0);
            if wrapEmpty.isSome() { return 11 }

            // Test subscript(clamped:) with negative index
            let clampNeg = arr(clamped: -5);
            if clampNeg.isNone() { return 12 }
            if clampNeg.unwrap() != 10 { return 13 }

            // Test subscript(clamped:) with over index
            let clampOver = arr(clamped: 100);
            if clampOver.isNone() { return 14 }
            if clampOver.unwrap() != 30 { return 15 }

            // Test subscript(clamped:) with normal index
            let clampNormal = arr(clamped: 1);
            if clampNormal.isNone() { return 16 }
            if clampNormal.unwrap() != 25 { return 17 }

            // Test subscript(clamped:) on empty array
            let clampEmpty = emptyArr(clamped: 0);
            if clampEmpty.isSome() { return 18 }

            // Test isValidIndex
            if arr.isValidIndex(0) == false { return 19 }
            if arr.isValidIndex(2) == false { return 20 }
            if arr.isValidIndex(3) { return 21 }
            if arr.isValidIndex(-1) { return 22 }

            // Test unchecked subscript setter
            arr(unchecked: 0) = 99;
            if arr(0) != 99 { return 23 }

            0
        }
