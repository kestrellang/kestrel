// test: execution
// stdlib: true

module Test

        func approxEqual(a: std.num.Float64, b: std.num.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.000001
        }

        func main() -> lang.i64 {
            let zero: std.num.Float64 = 0.0;
            let one: std.num.Float64 = 1.0;

            // sinh: sinh(0) = 0
            if approxEqual(zero.sinh(), 0.0) == false { return 1 }
            // sinh(1) ~ 1.1752011936438014
            let sinh1: std.num.Float64 = 1.1752011936438014;
            if approxEqual(one.sinh(), sinh1) == false { return 2 }

            // cosh: cosh(0) = 1
            if approxEqual(zero.cosh(), 1.0) == false { return 3 }
            // cosh(1) ~ 1.5430806348152437
            let cosh1: std.num.Float64 = 1.5430806348152437;
            if approxEqual(one.cosh(), cosh1) == false { return 4 }

            // tanh: tanh(0) = 0
            if approxEqual(zero.tanh(), 0.0) == false { return 5 }
            // tanh(inf) = 1
            let inf = std.num.Float64.infinity;
            if approxEqual(inf.tanh(), 1.0) == false { return 6 }

            // asinh: asinh(0) = 0
            if approxEqual(zero.asinh(), 0.0) == false { return 7 }
            // asinh(sinh(1)) = 1 (round-trip)
            if approxEqual(one.sinh().asinh(), 1.0) == false { return 8 }

            // acosh: acosh(1) = 0
            if approxEqual(one.acosh(), 0.0) == false { return 9 }
            // acosh(cosh(1)) = 1 (round-trip)
            if approxEqual(one.cosh().acosh(), 1.0) == false { return 10 }
            // acosh(0.5) = NaN (domain error)
            let half: std.num.Float64 = 0.5;
            if half.acosh().isNaN == false { return 11 }

            // atanh: atanh(0) = 0
            if approxEqual(zero.atanh(), 0.0) == false { return 12 }
            // atanh(tanh(0.5)) = 0.5 (round-trip)
            if approxEqual(half.tanh().atanh(), 0.5) == false { return 13 }
            // atanh(1) = infinity
            if one.atanh().isInfinite == false { return 14 }

            0
        }
