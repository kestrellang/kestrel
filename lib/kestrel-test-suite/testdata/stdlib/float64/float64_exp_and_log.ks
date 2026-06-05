// test: execution
// stdlib: true

module Test

        func approxEqual(a: std.numeric.Float64, b: std.numeric.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.000001
        }

        @main
        func main() -> lang.i64 {
            // exp: e^0 = 1, e^1 = e
            let zero: std.numeric.Float64 = 0.0;
            if approxEqual(zero.exp(), 1.0) == false { return 1 }
            let one: std.numeric.Float64 = 1.0;
            if approxEqual(one.exp(), std.numeric.Float64.e) == false { return 2 }

            // exp2: 2^0 = 1, 2^3 = 8
            if approxEqual(zero.exp2(), 1.0) == false { return 3 }
            let three: std.numeric.Float64 = 3.0;
            if approxEqual(three.exp2(), 8.0) == false { return 4 }

            // expm1: e^0 - 1 = 0, e^1 - 1 = e - 1
            if approxEqual(zero.expm1(), 0.0) == false { return 5 }
            let expm1One = one.expm1();
            let eMinusOne = std.numeric.Float64.e.subtract(1.0);
            if approxEqual(expm1One, eMinusOne) == false { return 6 }

            // ln: ln(1) = 0, ln(e) = 1
            if approxEqual(one.ln(), 0.0) == false { return 7 }
            let e = std.numeric.Float64.e;
            if approxEqual(e.ln(), 1.0) == false { return 8 }

            // ln1p: ln1p(0) = 0
            if approxEqual(zero.ln1p(), 0.0) == false { return 9 }
            // ln1p(1) = ln(2)
            if approxEqual(one.ln1p(), std.numeric.Float64.ln2) == false { return 10 }

            // log2: log2(1) = 0, log2(8) = 3
            if approxEqual(one.log2(), 0.0) == false { return 11 }
            let eight: std.numeric.Float64 = 8.0;
            if approxEqual(eight.log2(), 3.0) == false { return 12 }

            // log10: log10(1) = 0, log10(100) = 2
            if approxEqual(one.log10(), 0.0) == false { return 13 }
            let hundred: std.numeric.Float64 = 100.0;
            if approxEqual(hundred.log10(), 2.0) == false { return 14 }

            // log(base:): log_2(8) = 3
            let logResult = eight.log(2.0);
            if approxEqual(logResult, 3.0) == false { return 15 }
            // log_3(81) = 4
            let eightyOne: std.numeric.Float64 = 81.0;
            if approxEqual(eightyOne.log(3.0), 4.0) == false { return 16 }

            // pow: 2^10 = 1024
            let two: std.numeric.Float64 = 2.0;
            if approxEqual(two.pow(10.0), 1024.0) == false { return 17 }
            // pow: 2^0.5 = sqrt(2)
            if approxEqual(two.pow(0.5), std.numeric.Float64.sqrt2) == false { return 18 }

            // powi: 2^10 = 1024
            let intTen: std.numeric.Int64 = 10;
            if approxEqual(two.powi(intTen), 1024.0) == false { return 19 }
            // powi: 2^-1 = 0.5
            let negOneInt: std.numeric.Int64 = -1;
            if approxEqual(two.powi(negOneInt), 0.5) == false { return 20 }

            // ln of negative number is NaN
            let negOne: std.numeric.Float64 = -1.0;
            if negOne.ln().isNaN == false { return 21 }

            0
        }
