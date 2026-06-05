// test: execution
// stdlib: true

module Test

        func approxEqual(a: std.numeric.Float64, b: std.numeric.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        @main
        func main() -> lang.i64 {
            let pi = std.numeric.Float64.pi;
            let halfPi = pi.divide(2.0);
            let quarterPi = pi.divide(4.0);
            let zero: std.numeric.Float64 = 0.0;
            let one: std.numeric.Float64 = 1.0;

            // sin: sin(0) = 0, sin(pi/2) = 1
            if approxEqual(zero.sin(), 0.0) == false { return 1 }
            if approxEqual(halfPi.sin(), 1.0) == false { return 2 }

            // cos: cos(0) = 1, cos(pi) = -1
            if approxEqual(zero.cos(), 1.0) == false { return 3 }
            if approxEqual(pi.cos(), -1.0) == false { return 4 }

            // tan: tan(0) = 0, tan(pi/4) = 1
            if approxEqual(zero.tan(), 0.0) == false { return 5 }
            if approxEqual(quarterPi.tan(), 1.0) == false { return 6 }

            // asin: asin(0) = 0, asin(1) = pi/2
            if approxEqual(zero.asin(), 0.0) == false { return 7 }
            if approxEqual(one.asin(), halfPi) == false { return 8 }

            // acos: acos(1) = 0, acos(0) = pi/2
            if approxEqual(one.acos(), 0.0) == false { return 9 }
            if approxEqual(zero.acos(), halfPi) == false { return 10 }

            // acos(-1) = pi
            let negOne: std.numeric.Float64 = -1.0;
            if approxEqual(negOne.acos(), pi) == false { return 11 }

            // atan: atan(0) = 0, atan(1) = pi/4
            if approxEqual(zero.atan(), 0.0) == false { return 12 }
            if approxEqual(one.atan(), quarterPi) == false { return 13 }

            // atan2: atan2(1, 1) = pi/4
            if approxEqual(one.atan2(1.0), quarterPi) == false { return 14 }
            // atan2(1, -1) = 3*pi/4
            let threePiOverFour = pi.multiply(3.0).divide(4.0);
            if approxEqual(one.atan2(-1.0), threePiOverFour) == false { return 15 }

            // sinCos: sin and cos should match individual calls
            let angle: std.numeric.Float64 = 1.0;
            let (s, c) = angle.sinCos();
            if approxEqual(s, angle.sin()) == false { return 16 }
            if approxEqual(c, angle.cos()) == false { return 17 }

            // asin(2) should be NaN (out of domain)
            let two: std.numeric.Float64 = 2.0;
            if two.asin().isNaN == false { return 18 }

            0
        }
