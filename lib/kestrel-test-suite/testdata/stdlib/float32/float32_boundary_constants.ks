// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // maxValue — 3.4028235e38
            let maxVal = std.numeric.Float32.maxValue;
            if maxVal.isFinite == false { return 1 }
            if maxVal.isPositive == false { return 2 }
            // maxValue should be greater than 3.0e38
            let threshold: std.numeric.Float32 = 3.0e38;
            if maxVal < threshold { return 3 }

            // minValue — -3.4028235e38
            let minVal = std.numeric.Float32.minValue;
            if minVal.isFinite == false { return 4 }
            if minVal.isNegative == false { return 5 }
            // minValue should be the negation of maxValue
            if minVal.negate().isEqual(to: maxVal) == false { return 6 }

            // minPositive — 1.17549435e-38 (smallest normal)
            let minPos = std.numeric.Float32.minPositive;
            if minPos.isPositive == false { return 7 }
            if minPos.isNormal == false { return 8 }

            // A value just below minPositive should be subnormal
            let two: std.numeric.Float32 = 2.0;
            let halfMinPos = minPos.divide(two);
            if halfMinPos.isSubnormal == false { return 9 }
            if halfMinPos.isNormal { return 10 }

            // epsilon — 1.1920929e-7
            let eps = std.numeric.Float32.epsilon;
            if eps.isPositive == false { return 11 }
            // 1.0 + epsilon should not equal 1.0
            let one: std.numeric.Float32 = 1.0;
            let onePlusEps = one.add(eps);
            if onePlusEps.isEqual(to: one) { return 12 }

            // Classification near subnormal boundary
            // minPositive itself is normal
            if minPos.isNormal == false { return 13 }
            if minPos.isSubnormal { return 14 }
            // nextDown from minPositive should be subnormal
            let belowMinPos = minPos.nextDown();
            if belowMinPos.isSubnormal == false { return 15 }
            if belowMinPos.isNormal { return 16 }
            // The subnormal value should still be finite and positive
            if belowMinPos.isFinite == false { return 17 }
            if belowMinPos.isPositive == false { return 18 }

            0
        }
