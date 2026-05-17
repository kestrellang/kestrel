// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // minValue is 0
            let minVal = std.numeric.UInt16.minValue;
            let lit0: std.numeric.UInt16 = 0;
            if minVal.isEqual(to: lit0) == false { return 1 }

            // maxValue is 65535
            let maxVal = std.numeric.UInt16.maxValue;
            let lit65535: std.numeric.UInt16 = 65535;
            if maxVal.isEqual(to: lit65535) == false { return 2 }

            // bitWidth is 16
            if std.numeric.UInt16.bitWidth != 16 { return 3 }

            // zero constant
            let z = std.numeric.UInt16.zero;
            let zeroLit: std.numeric.UInt16 = 0;
            if z.isEqual(to: zeroLit) == false { return 4 }

            // one constant
            let o = std.numeric.UInt16.one;
            let oneLit: std.numeric.UInt16 = 1;
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
