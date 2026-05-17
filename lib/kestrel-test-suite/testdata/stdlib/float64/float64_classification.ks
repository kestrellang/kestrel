// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let normal: std.numeric.Float64 = 1.0;
            let zero = std.numeric.Float64.zero;
            let inf = std.numeric.Float64.infinity;
            let nan = std.numeric.Float64.nan;
            let negInf = inf.negate();

            // isNaN
            if nan.isNaN == false { return 1 }
            if normal.isNaN { return 2 }
            if inf.isNaN { return 3 }
            if zero.isNaN { return 4 }

            // isInfinite
            if inf.isInfinite == false { return 5 }
            if negInf.isInfinite == false { return 6 }
            if normal.isInfinite { return 7 }
            if nan.isInfinite { return 8 }

            // isFinite
            if normal.isFinite == false { return 9 }
            if zero.isFinite == false { return 10 }
            if inf.isFinite { return 11 }
            if nan.isFinite { return 12 }

            // isNormal
            if normal.isNormal == false { return 13 }
            if zero.isNormal { return 14 }
            if inf.isNormal { return 15 }
            if nan.isNormal { return 16 }

            // isSubnormal - minPositive / 2 should be subnormal
            let minPos = std.numeric.Float64.minPositive;
            let subnormal = minPos.divide(2.0);
            if subnormal.isSubnormal == false { return 17 }
            if normal.isSubnormal { return 18 }
            if zero.isSubnormal { return 19 }
            if minPos.isSubnormal { return 20 }

            0
        }
