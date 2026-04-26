// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // minValue is 0
            let minVal = std.num.UInt32.minValue;
            let lit0: std.num.UInt32 = 0;
            if minVal.equals(lit0) == false { return 1 }

            // maxValue is 4294967295
            let maxVal = std.num.UInt32.maxValue;
            let lit4294967295: std.num.UInt32 = 4294967295;
            if maxVal.equals(lit4294967295) == false { return 2 }

            // bitWidth is 32
            if std.num.UInt32.bitWidth != 32 { return 3 }

            // zero constant
            let z = std.num.UInt32.zero;
            let zeroLit: std.num.UInt32 = 0;
            if z.equals(zeroLit) == false { return 4 }

            // one constant
            let o = std.num.UInt32.one;
            let oneLit: std.num.UInt32 = 1;
            if o.equals(oneLit) == false { return 5 }

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
