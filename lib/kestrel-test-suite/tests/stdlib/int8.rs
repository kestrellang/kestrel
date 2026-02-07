use kestrel_test_suite::*;

#[test]
fn int8_boundaries_and_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // minValue should be -128
            let minVal = std.num.Int8.minValue;
            let minAsI64 = std.num.Int64(from: minVal);
            if minAsI64 != -128 { return 1 }

            // maxValue should be 127
            let maxVal = std.num.Int8.maxValue;
            let maxAsI64 = std.num.Int64(from: maxVal);
            if maxAsI64 != 127 { return 2 }

            // bitWidth should be 8
            if std.num.Int8.bitWidth != 8 { return 3 }

            // minValue is negative
            if minVal.isNegative == false { return 4 }
            // maxValue is positive
            if maxVal.isPositive == false { return 5 }

            // zero
            let zero = std.num.Int8.zero;
            if zero.isZero == false { return 6 }
            let zeroAsI64 = std.num.Int64(from: zero);
            if zeroAsI64 != 0 { return 7 }

            // one
            let one = std.num.Int8.one;
            let oneAsI64 = std.num.Int64(from: one);
            if oneAsI64 != 1 { return 8 }

            // Verify minValue + maxValue = -1 (wrapping)
            let sum = minVal.add(maxVal);
            let sumAsI64 = std.num.Int64(from: sum);
            if sumAsI64 != -1 { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int8_overflow_behavior() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let maxVal = std.num.Int8.maxValue;
            let minVal = std.num.Int8.minValue;
            let one: std.num.Int8 = 1;
            let negOne: std.num.Int8 = -1;

            // addChecked — overflow at 127
            let addCheckZero: std.num.Int8 = 0;
            let addOk = maxVal.addChecked(addCheckZero);
            if addOk.isNone() { return 1 }
            let addOverflow = maxVal.addChecked(one);
            if addOverflow.isSome() { return 2 }

            // addChecked — normal case
            let ten: std.num.Int8 = 10;
            let five: std.num.Int8 = 5;
            let addNormal = ten.addChecked(five);
            if addNormal.isNone() { return 3 }
            let expectedFifteen: std.num.Int8 = 15;
            if addNormal.unwrap() != expectedFifteen { return 4 }

            // subtractChecked — underflow at -128
            let subOverflow = minVal.subtractChecked(one);
            if subOverflow.isSome() { return 5 }

            // subtractChecked — normal case
            let subNormal = ten.subtractChecked(five);
            if subNormal.isNone() { return 6 }
            let expectedFive: std.num.Int8 = 5;
            if subNormal.unwrap() != expectedFive { return 7 }

            // multiplyChecked — overflow near boundaries
            let big: std.num.Int8 = 100;
            let two: std.num.Int8 = 2;
            let mulOverflow = big.multiplyChecked(two);
            if mulOverflow.isSome() { return 8 }

            // multiplyChecked — normal case
            let three: std.num.Int8 = 3;
            let mulNormal = five.multiplyChecked(three);
            if mulNormal.isNone() { return 9 }
            let expectedMulFifteen: std.num.Int8 = 15;
            if mulNormal.unwrap() != expectedMulFifteen { return 10 }

            // negateChecked — overflow at -128 (no positive 128 in Int8)
            let negMin = minVal.negateChecked();
            if negMin.isSome() { return 11 }

            // negateChecked — normal case
            let negTen = ten.negateChecked();
            if negTen.isNone() { return 12 }
            let expectedNegTen: std.num.Int8 = -10;
            if negTen.unwrap() != expectedNegTen { return 13 }

            // absChecked — overflow at -128
            let absMin = minVal.absChecked();
            if absMin.isSome() { return 14 }

            // absChecked — normal case
            let negFive: std.num.Int8 = -5;
            let absFive = negFive.absChecked();
            if absFive.isNone() { return 15 }
            if absFive.unwrap() != five { return 16 }

            // addSaturating — clamps to 127
            let addSat = maxVal.addSaturating(one);
            if addSat != maxVal { return 17 }
            let satHundred: std.num.Int8 = 100;
            let addSatBig = maxVal.addSaturating(satHundred);
            if addSatBig != maxVal { return 18 }

            // addSaturating — clamps to -128
            let addSatNeg = minVal.addSaturating(negOne);
            if addSatNeg != minVal { return 19 }

            // subtractSaturating — clamps to -128
            let subSat = minVal.subtractSaturating(one);
            if subSat != minVal { return 20 }

            // subtractSaturating — clamps to 127
            let subSatPos = maxVal.subtractSaturating(negOne);
            if subSatPos != maxVal { return 21 }

            // multiplySaturating — clamps to 127
            let mulSat = big.multiplySaturating(two);
            if mulSat != maxVal { return 22 }

            // multiplySaturating — clamps to -128 (positive * negative overflow)
            let negBig: std.num.Int8 = -100;
            let mulSatNeg = negBig.multiplySaturating(two);
            if mulSatNeg != minVal { return 23 }

            // negateSaturating — -128 saturates to 127
            let negSatMin = minVal.negateSaturating();
            if negSatMin != maxVal { return 24 }

            // negateSaturating — normal case
            let negSatTen = ten.negateSaturating();
            let expectedNegSatTen: std.num.Int8 = -10;
            if negSatTen != expectedNegSatTen { return 25 }

            // absSaturating — -128 saturates to 127
            let absSatMin = minVal.absSaturating();
            if absSatMin != maxVal { return 26 }

            // absSaturating — normal case
            let absSatNeg = negFive.absSaturating();
            if absSatNeg != five { return 27 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int8_bitwidth_and_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // byteSwapped — identity for single-byte type
            let val: std.num.Int8 = 42;
            if val.byteSwapped != val { return 1 }
            let negVal: std.num.Int8 = -42;
            if negVal.byteSwapped != negVal { return 2 }

            // leadingZeros — relative to 8-bit width
            let one: std.num.Int8 = 1;
            if one.leadingZeros != 7 { return 3 }
            let zero: std.num.Int8 = 0;
            if zero.leadingZeros != 8 { return 4 }
            // -1 in Int8 is all 1s, so 0 leading zeros
            let negOne: std.num.Int8 = -1;
            if negOne.leadingZeros != 0 { return 5 }
            let four: std.num.Int8 = 4;
            // 4 = 0b00000100 -> 5 leading zeros
            if four.leadingZeros != 5 { return 6 }

            // trailingZeros — relative to 8-bit width
            if one.trailingZeros != 0 { return 7 }
            if zero.trailingZeros != 8 { return 8 }
            let eight: std.num.Int8 = 8;
            // 8 = 0b00001000 -> 3 trailing zeros
            if eight.trailingZeros != 3 { return 9 }

            // rotateLeft — 8-bit rotation
            // rotate 1 left by 1 = 2
            let expectedRotateTwo: std.num.Int8 = 2;
            if one.rotateLeft(by: 1) != expectedRotateTwo { return 10 }
            // rotate by 0 = unchanged
            if val.rotateLeft(by: 0) != val { return 11 }

            // rotateRight — 8-bit rotation
            // rotate 2 right by 1 = 1
            let two: std.num.Int8 = 2;
            if two.rotateRight(by: 1) != one { return 12 }
            // rotate by 0 = unchanged
            if val.rotateRight(by: 0) != val { return 13 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.num.Int8 = 37;
            if testVal.rotateLeft(by: 3).rotateRight(by: 3) != testVal { return 14 }

            // init(from:) — from Int64
            let i64val: std.num.Int64 = 100;
            let fromI64 = std.num.Int8(from: i64val);
            let expectedHundred: std.num.Int8 = 100;
            if fromI64 != expectedHundred { return 15 }

            // init(from:) — from Int64 negative
            let negI64: std.num.Int64 = -50;
            let fromNegI64 = std.num.Int8(from: negI64);
            let expectedNegFifty: std.num.Int8 = -50;
            if fromNegI64 != expectedNegFifty { return 16 }

            // parse — valid Int8 value
            let parsed = std.num.Int8.parse( "42");
            if parsed.isNone() { return 17 }
            if parsed.unwrap() != val { return 18 }

            // parse — negative value
            let parsedNeg = std.num.Int8.parse( "-128");
            if parsedNeg.isNone() { return 19 }
            if parsedNeg.unwrap() != std.num.Int8.minValue { return 20 }

            // parse — maxValue
            let parsedMax = std.num.Int8.parse( "127");
            if parsedMax.isNone() { return 21 }
            if parsedMax.unwrap() != std.num.Int8.maxValue { return 22 }

            // parse — out-of-range (too large)
            let parsedBig = std.num.Int8.parse( "128");
            if parsedBig.isSome() { return 23 }

            // parse — out-of-range (too small)
            let parsedSmall = std.num.Int8.parse( "-129");
            if parsedSmall.isSome() { return 24 }

            // parse — invalid string
            let parsedBad = std.num.Int8.parse( "abc");
            if parsedBad.isSome() { return 25 }

            // parse — empty string
            let parsedEmpty = std.num.Int8.parse( "");
            if parsedEmpty.isSome() { return 26 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
