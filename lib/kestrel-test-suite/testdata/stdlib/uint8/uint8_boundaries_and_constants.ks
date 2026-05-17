// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // minValue is 0
            let minVal = std.numeric.UInt8.minValue;
            let lit0: std.numeric.UInt8 = 0;
            if minVal.isEqual(to: lit0) == false { return 1 }

            // maxValue is 255
            let maxVal = std.numeric.UInt8.maxValue;
            let lit255: std.numeric.UInt8 = 255;
            if maxVal.isEqual(to: lit255) == false { return 2 }

            // bitWidth is 8
            if std.numeric.UInt8.bitWidth != 8 { return 3 }

            // zero constant
            let z = std.numeric.UInt8.zero;
            let zeroLit: std.numeric.UInt8 = 0;
            if z.isEqual(to: zeroLit) == false { return 4 }

            // one constant
            let o = std.numeric.UInt8.one;
            let oneLit: std.numeric.UInt8 = 1;
            if o.isEqual(to: oneLit) == false { return 5 }

            // isZero
            if minVal.isZero == false { return 6 }
            if maxVal.isZero { return 7 }

            // isPositive
            if maxVal.isPositive == false { return 8 }
            if minVal.isPositive { return 9 }

            // isNegative is always false for unsigned
            if minVal.isNegative { return 10 }
            if maxVal.isNegative { return 11 }

            0
        }
