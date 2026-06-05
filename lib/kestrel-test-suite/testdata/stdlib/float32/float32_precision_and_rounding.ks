// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // round() near maxValue — should not overflow
            let maxVal = std.numeric.Float32.maxValue;
            let rounded = maxVal.round();
            if rounded.isFinite == false { return 1 }
            if rounded.isEqual(to: maxVal) == false { return 2 }

            // trunc() near maxValue — should return maxValue (already integer)
            let truncated = maxVal.trunc();
            if truncated.isFinite == false { return 3 }
            if truncated.isEqual(to: maxVal) == false { return 4 }

            // trunc() on a large positive float
            let large: std.numeric.Float32 = 1.0e30;
            let truncLarge = large.trunc();
            if truncLarge.isEqual(to: large) == false { return 5 }

            // Precision in trig: sin(pi) should be very close to 0 but may not be exactly 0
            let pi = std.numeric.Float32.pi;
            let sinPi = pi.sin();
            // For Float32, sin(pi) is approximate. Check that abs < epsilon-ish (1e-6)
            let tolerance: std.numeric.Float32 = 1.0e-6;
            if sinPi.abs() > tolerance { return 6 }

            // cos(0) should be exactly 1.0
            let zero: std.numeric.Float32 = 0.0;
            let cosZero = zero.cos();
            let oneF: std.numeric.Float32 = 1.0;
            if cosZero.isEqual(to: oneF) == false { return 7 }

            // Float32 precision: adding a small value to a large value
            // 1e7 + 1.0 should still work in Float32 (within 24-bit mantissa)
            let bigVal: std.numeric.Float32 = 1.0e7;
            let bigOneF: std.numeric.Float32 = 1.0;
            let bigPlusOne = bigVal.add(bigOneF);
            let expected: std.numeric.Float32 = 10000001.0;
            if bigPlusOne.isEqual(to: expected) == false { return 8 }

            // But 1e8 + 1.0 may lose the +1 due to Float32 precision limits
            let veryBig: std.numeric.Float32 = 1.0e8;
            let veryBigOneF: std.numeric.Float32 = 1.0;
            let veryBigPlusOne = veryBig.add(veryBigOneF);
            // The 1.0 is lost in Float32 precision, so result == veryBig
            if veryBigPlusOne.isEqual(to: veryBig) == false { return 9 }

            0
        }
