use kestrel_test_suite::*;

#[test]
fn int16_boundaries_and_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // minValue should be -32768
            let minVal = std.num.Int16.minValue;
            let minAsI64 = std.num.Int64(from: minVal);
            if minAsI64 != -32768 { return 1 }

            // maxValue should be 32767
            let maxVal = std.num.Int16.maxValue;
            let maxAsI64 = std.num.Int64(from: maxVal);
            if maxAsI64 != 32767 { return 2 }

            // bitWidth should be 16
            if std.num.Int16.bitWidth != 16 { return 3 }

            // minValue is negative
            if minVal.isNegative == false { return 4 }
            // maxValue is positive
            if maxVal.isPositive == false { return 5 }

            // zero
            let zero = std.num.Int16.zero;
            if zero.isZero == false { return 6 }
            let zeroAsI64 = std.num.Int64(from: zero);
            if zeroAsI64 != 0 { return 7 }

            // one
            let one = std.num.Int16.one;
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
fn int16_overflow_behavior() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let maxVal = std.num.Int16.maxValue;
            let minVal = std.num.Int16.minValue;
            let one: std.num.Int16 = 1;
            let negOne: std.num.Int16 = -1;

            // addChecked — overflow at 32767
            let addOverflow = maxVal.addChecked(one);
            if addOverflow.isSome() { return 1 }

            // addChecked — normal case
            let hundred: std.num.Int16 = 100;
            let fifty: std.num.Int16 = 50;
            let addNormal = hundred.addChecked(fifty);
            if addNormal.isNone() { return 2 }
            let expectedAdd: std.num.Int16 = 150;
            if addNormal.unwrap() != expectedAdd { return 3 }

            // subtractChecked — underflow at -32768
            let subOverflow = minVal.subtractChecked(one);
            if subOverflow.isSome() { return 4 }

            // subtractChecked — normal case
            let subNormal = hundred.subtractChecked(fifty);
            if subNormal.isNone() { return 5 }
            let expectedSub: std.num.Int16 = 50;
            if subNormal.unwrap() != expectedSub { return 6 }

            // multiplyChecked — overflow near boundaries
            let big: std.num.Int16 = 20000;
            let two: std.num.Int16 = 2;
            let mulOverflow = big.multiplyChecked(two);
            if mulOverflow.isSome() { return 7 }

            // multiplyChecked — normal case
            let ten: std.num.Int16 = 10;
            let five: std.num.Int16 = 5;
            let mulNormal = ten.multiplyChecked(five);
            if mulNormal.isNone() { return 8 }
            let expectedMul: std.num.Int16 = 50;
            if mulNormal.unwrap() != expectedMul { return 9 }

            // negateChecked — overflow at -32768
            let negMin = minVal.negateChecked();
            if negMin.isSome() { return 10 }

            // negateChecked — normal case
            let negHundred = hundred.negateChecked();
            if negHundred.isNone() { return 11 }
            let expectedNeg: std.num.Int16 = -100;
            if negHundred.unwrap() != expectedNeg { return 12 }

            // absChecked — overflow at -32768
            let absMin = minVal.absChecked();
            if absMin.isSome() { return 13 }

            // absChecked — normal case
            let negFifty: std.num.Int16 = -50;
            let absFifty = negFifty.absChecked();
            if absFifty.isNone() { return 14 }
            if absFifty.unwrap() != fifty { return 15 }

            // addSaturating — clamps to 32767
            let addSat = maxVal.addSaturating(one);
            if addSat != maxVal { return 16 }
            let thousand: std.num.Int16 = 1000;
            let addSatBig = maxVal.addSaturating(thousand);
            if addSatBig != maxVal { return 17 }

            // addSaturating — clamps to -32768
            let addSatNeg = minVal.addSaturating(negOne);
            if addSatNeg != minVal { return 18 }

            // subtractSaturating — clamps to -32768
            let subSat = minVal.subtractSaturating(one);
            if subSat != minVal { return 19 }

            // subtractSaturating — clamps to 32767
            let subSatPos = maxVal.subtractSaturating(negOne);
            if subSatPos != maxVal { return 20 }

            // multiplySaturating — clamps to 32767
            let mulSat = big.multiplySaturating(two);
            if mulSat != maxVal { return 21 }

            // multiplySaturating — clamps to -32768 (positive * negative overflow)
            let negBig: std.num.Int16 = -20000;
            let mulSatNeg = negBig.multiplySaturating(two);
            if mulSatNeg != minVal { return 22 }

            // negateSaturating — -32768 saturates to 32767
            let negSatMin = minVal.negateSaturating();
            if negSatMin != maxVal { return 23 }

            // negateSaturating — normal case
            let negSatHundred = hundred.negateSaturating();
            let expectedNegSat: std.num.Int16 = -100;
            if negSatHundred != expectedNegSat { return 24 }

            // absSaturating — -32768 saturates to 32767
            let absSatMin = minVal.absSaturating();
            if absSatMin != maxVal { return 25 }

            // absSaturating — normal case
            let absSatNeg = negFifty.absSaturating();
            if absSatNeg != fifty { return 26 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn int16_bitwidth_and_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // byteSwapped — 2-byte swap
            // 0x0102 = 258 -> byte swapped = 0x0201 = 513
            let val: std.num.Int16 = 258;
            let swapped = val.byteSwapped;
            let swappedAsI64 = std.num.Int64(from: swapped);
            if swappedAsI64 != 513 { return 1 }

            // byteSwapped — 1 in Int16 is 0x0001, swapped is 0x0100 = 256
            let one: std.num.Int16 = 1;
            let oneSwapped = one.byteSwapped;
            let oneSwappedI64 = std.num.Int64(from: oneSwapped);
            if oneSwappedI64 != 256 { return 2 }

            // leadingZeros — relative to 16-bit width
            if one.leadingZeros != 15 { return 3 }
            let zero: std.num.Int16 = 0;
            if zero.leadingZeros != 16 { return 4 }
            // -1 in Int16 is all 1s, so 0 leading zeros
            let negOne: std.num.Int16 = -1;
            if negOne.leadingZeros != 0 { return 5 }
            // 256 = 0x0100 = 0b0000000100000000 -> 7 leading zeros
            let val256: std.num.Int16 = 256;
            if val256.leadingZeros != 7 { return 6 }

            // rotateLeft — 16-bit rotation
            let expectedRotateLeft: std.num.Int16 = 2;
            if one.rotateLeft(by: 1) != expectedRotateLeft { return 7 }
            let fortyTwo: std.num.Int16 = 42;
            if fortyTwo.rotateLeft(by: 0) != fortyTwo { return 8 }

            // rotateRight — 16-bit rotation
            let two: std.num.Int16 = 2;
            if two.rotateRight(by: 1) != one { return 9 }
            if fortyTwo.rotateRight(by: 0) != fortyTwo { return 10 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.num.Int16 = 12345;
            if testVal.rotateLeft(by: 5).rotateRight(by: 5) != testVal { return 11 }

            // init(from:) — from Int64
            let i64val: std.num.Int64 = 1000;
            let fromI64 = std.num.Int16(from: i64val);
            let expectedFromI64: std.num.Int16 = 1000;
            if fromI64 != expectedFromI64 { return 12 }

            // init(from:) — from Int8
            let i8val: std.num.Int8 = -50;
            let fromI8 = std.num.Int16(from: i8val);
            let expectedFromI8: std.num.Int16 = -50;
            if fromI8 != expectedFromI8 { return 13 }

            // parse — valid Int16 value
            let parsed = std.num.Int16.parse( "1000");
            if parsed.isNone() { return 14 }
            let expectedParsed: std.num.Int16 = 1000;
            if parsed.unwrap() != expectedParsed { return 15 }

            // parse — negative value
            let parsedNeg = std.num.Int16.parse( "-32768");
            if parsedNeg.isNone() { return 16 }
            if parsedNeg.unwrap() != std.num.Int16.minValue { return 17 }

            // parse — maxValue
            let parsedMax = std.num.Int16.parse( "32767");
            if parsedMax.isNone() { return 18 }
            if parsedMax.unwrap() != std.num.Int16.maxValue { return 19 }

            // parse — out-of-range (too large)
            let parsedBig = std.num.Int16.parse( "32768");
            if parsedBig.isSome() { return 20 }

            // parse — out-of-range (too small)
            let parsedSmall = std.num.Int16.parse( "-32769");
            if parsedSmall.isSome() { return 21 }

            // parse — invalid string
            let parsedBad = std.num.Int16.parse( "xyz");
            if parsedBad.isSome() { return 22 }

            // parse — empty string
            let parsedEmpty = std.num.Int16.parse( "");
            if parsedEmpty.isSome() { return 23 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
