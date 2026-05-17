// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // minValue should be -2147483648
            let minVal = std.numeric.Int32.minValue;
            let minAsI64 = std.numeric.Int64(from: minVal);
            if minAsI64 != -2147483648 { return 1 }

            // maxValue should be 2147483647
            let maxVal = std.numeric.Int32.maxValue;
            let maxAsI64 = std.numeric.Int64(from: maxVal);
            if maxAsI64 != 2147483647 { return 2 }

            // bitWidth should be 32
            if std.numeric.Int32.bitWidth != 32 { return 3 }

            // minValue is negative
            if minVal.isNegative == false { return 4 }
            // maxValue is positive
            if maxVal.isPositive == false { return 5 }

            // zero
            let zero = std.numeric.Int32.zero;
            if zero.isZero == false { return 6 }
            let zeroAsI64 = std.numeric.Int64(from: zero);
            if zeroAsI64 != 0 { return 7 }

            // one
            let one = std.numeric.Int32.one;
            let oneAsI64 = std.numeric.Int64(from: one);
            if oneAsI64 != 1 { return 8 }

            // Verify minValue + maxValue = -1 (wrapping)
            let sum = minVal.add(maxVal);
            let sumAsI64 = std.numeric.Int64(from: sum);
            if sumAsI64 != -1 { return 9 }

            0
        }
