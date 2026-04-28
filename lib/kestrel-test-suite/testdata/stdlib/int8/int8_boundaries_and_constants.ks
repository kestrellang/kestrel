// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // minValue should be -128
            let minVal = std.numeric.Int8.minValue;
            let minAsI64 = std.numeric.Int64(from: minVal);
            if minAsI64 != -128 { return 1 }

            // maxValue should be 127
            let maxVal = std.numeric.Int8.maxValue;
            let maxAsI64 = std.numeric.Int64(from: maxVal);
            if maxAsI64 != 127 { return 2 }

            // bitWidth should be 8
            if std.numeric.Int8.bitWidth != 8 { return 3 }

            // minValue is negative
            if minVal.isNegative == false { return 4 }
            // maxValue is positive
            if maxVal.isPositive == false { return 5 }

            // zero
            let zero = std.numeric.Int8.zero;
            if zero.isZero == false { return 6 }
            let zeroAsI64 = std.numeric.Int64(from: zero);
            if zeroAsI64 != 0 { return 7 }

            // one
            let one = std.numeric.Int8.one;
            let oneAsI64 = std.numeric.Int64(from: one);
            if oneAsI64 != 1 { return 8 }

            // Verify minValue + maxValue = -1 (wrapping)
            let sum = minVal.add(maxVal);
            let sumAsI64 = std.numeric.Int64(from: sum);
            if sumAsI64 != -1 { return 9 }

            0
        }
