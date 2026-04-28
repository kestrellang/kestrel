// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // init(from: Float64) — narrowing conversion
            let f64val: std.numeric.Float64 = 3.14;
            let f32fromF64 = std.numeric.Float32(from: f64val);
            // Should be approximately 3.14 (Float32 precision)
            let f32ThreePtOneFour: std.numeric.Float32 = 3.14;
            let diff = f32fromF64.subtract(f32ThreePtOneFour).abs();
            let tolerance: std.numeric.Float32 = 1.0e-6;
            if diff > tolerance { return 1 }

            // init(from: Int64)
            let i64val: std.numeric.Int64 = 42;
            let f32fromI64 = std.numeric.Float32(from: i64val);
            let fortyTwoF: std.numeric.Float32 = 42.0;
            if f32fromI64.equals(fortyTwoF) == false { return 2 }

            // init(from: Int64) with negative
            let negI64: std.numeric.Int64 = -100;
            let f32fromNeg = std.numeric.Float32(from: negI64);
            let negHundredF: std.numeric.Float32 = -100.0;
            if f32fromNeg.equals(negHundredF) == false { return 3 }

            // toInt64 — normal case
            let f32val: std.numeric.Float32 = 3.7;
            let asInt = f32val.toInt64();
            if asInt.isNone() { return 4 }
            if asInt.unwrap() != 3 { return 5 }

            // toInt64 — negative truncation toward zero
            let negFloat: std.numeric.Float32 = -3.7;
            let negAsInt = negFloat.toInt64();
            if negAsInt.isNone() { return 6 }
            if negAsInt.unwrap() != -3 { return 7 }

            // toInt64 — NaN returns None
            let nan = std.numeric.Float32.nan;
            let nanAsInt = nan.toInt64();
            if nanAsInt.isSome() { return 8 }

            // toInt64 — infinity returns None
            let inf = std.numeric.Float32.infinity;
            let infAsInt = inf.toInt64();
            if infAsInt.isSome() { return 9 }

            // toFloat64 — widening conversion
            let small: std.numeric.Float32 = 1.5;
            let asF64 = small.toFloat64();
            // 1.5 is exactly representable in both Float32 and Float64
            let onePointFiveF64: std.numeric.Float64 = 1.5;
            if asF64.equals(onePointFiveF64) == false { return 10 }

            // parse — valid float
            let parsed = std.numeric.Float32.parse("3.14");
            if parsed.isNone() { return 11 }
            let parsedVal = parsed.unwrap();
            let parseThreePtOneFour: std.numeric.Float32 = 3.14;
            let parseDiff = parsedVal.subtract(parseThreePtOneFour).abs();
            if parseDiff > tolerance { return 12 }

            // parse — negative value
            let parsedNeg = std.numeric.Float32.parse("-2.5");
            if parsedNeg.isNone() { return 13 }
            let parsedNegVal = parsedNeg.unwrap();
            let negTwoPointFive: std.numeric.Float32 = -2.5;
            if parsedNegVal.equals(negTwoPointFive) == false { return 14 }

            // parse — scientific notation
            let parsedSci = std.numeric.Float32.parse("1.5e2");
            if parsedSci.isNone() { return 15 }
            let oneHundredFifty: std.numeric.Float32 = 150.0;
            if parsedSci.unwrap().equals(oneHundredFifty) == false { return 16 }

            // parse — "nan"
            let parsedNan = std.numeric.Float32.parse("nan");
            if parsedNan.isNone() { return 17 }
            if parsedNan.unwrap().isNaN == false { return 18 }

            // parse — "inf"
            let parsedInf = std.numeric.Float32.parse("inf");
            if parsedInf.isNone() { return 19 }
            if parsedInf.unwrap().isInfinite == false { return 20 }

            // parse — invalid string
            let parsedBad = std.numeric.Float32.parse("abc");
            if parsedBad.isSome() { return 21 }

            // parse — empty string
            let parsedEmpty = std.numeric.Float32.parse("");
            if parsedEmpty.isSome() { return 22 }

            // parse — integer string
            let parsedInt = std.numeric.Float32.parse("42");
            if parsedInt.isNone() { return 23 }
            let parsedFortyTwo: std.numeric.Float32 = 42.0;
            if parsedInt.unwrap().equals(parsedFortyTwo) == false { return 24 }

            0
        }
