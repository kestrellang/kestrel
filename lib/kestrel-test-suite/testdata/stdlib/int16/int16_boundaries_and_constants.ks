// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // minValue should be -32768
            let minVal = std.num.Int16.minValue;
            let minAsI64 = std.num.Int64(from: minVal);
            if minAsI64 != -32768 { return 1 }

            // maxValue should be 32767
            let maxVal = std.num.Int16.maxValue;
            let maxAsI64 = std.num.Int64(from: maxVal);
            if maxAsI64 != 32767 { return 2 }

            // bitWidth should be 16
            if std.num.Int16.bitWidth != 16 { return 3 }

            // minValue is negative
            if minVal.isNegative == false { return 4 }
            // maxValue is positive
            if maxVal.isPositive == false { return 5 }

            // zero
            let zero = std.num.Int16.zero;
            if zero.isZero == false { return 6 }
            let zeroAsI64 = std.num.Int64(from: zero);
            if zeroAsI64 != 0 { return 7 }

            // one
            let one = std.num.Int16.one;
            let oneAsI64 = std.num.Int64(from: one);
            if oneAsI64 != 1 { return 8 }

            // Verify minValue + maxValue = -1 (wrapping)
            let sum = minVal.add(maxVal);
            let sumAsI64 = std.num.Int64(from: sum);
            if sumAsI64 != -1 { return 9 }

            0
        }
