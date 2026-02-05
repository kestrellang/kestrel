use kestrel_test_suite::*;

#[test]
fn int32_boundaries_and_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // minValue should be -2147483648
            let minVal = std.num.Int32.minValue;
            let minAsI64 = std.num.Int64(from: minVal);
            if minAsI64 != -2147483648 { return 1 }

            // maxValue should be 2147483647
            let maxVal = std.num.Int32.maxValue;
            let maxAsI64 = std.num.Int64(from: maxVal);
            if maxAsI64 != 2147483647 { return 2 }

            // bitWidth should be 32
            if std.num.Int32.bitWidth != 32 { return 3 }

            // minValue is negative
            if minVal.isNegative == false { return 4 }
            // maxValue is positive
            if maxVal.isPositive == false { return 5 }

            // zero
            let zero = std.num.Int32.zero;
            if zero.isZero == false { return 6 }
            let zeroAsI64 = std.num.Int64(from: zero);
            if zeroAsI64 != 0 { return 7 }

            // one
            let one = std.num.Int32.one;
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

// TODO: Fails due to unary minus requiring Negatable protocol resolution on Int32
#[test]
fn int32_overflow_behavior() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let maxVal = std.num.Int32.maxValue;
            let minVal = std.num.Int32.minValue;
            let one = std.num.Int32(intLiteral: 1);
            let negOne = std.num.Int32(intLiteral: -1);

            // addChecked — overflow at 2147483647
            let addOverflow = maxVal.addChecked(one);
            if addOverflow.isSome() { return 1 }

            // addChecked — normal case
            let thousand = std.num.Int32(intLiteral: 1000);
            let fiveHundred = std.num.Int32(intLiteral: 500);
            let addNormal = thousand.addChecked(fiveHundred);
            if addNormal.isNone() { return 2 }
            if addNormal.unwrap() != std.num.Int32(intLiteral: 1500) { return 3 }

            // subtractChecked — underflow at -2147483648
            let subOverflow = minVal.subtractChecked(one);
            if subOverflow.isSome() { return 4 }

            // subtractChecked — normal case
            let subNormal = thousand.subtractChecked(fiveHundred);
            if subNormal.isNone() { return 5 }
            if subNormal.unwrap() != fiveHundred { return 6 }

            // multiplyChecked — overflow near boundaries
            let bigVal = std.num.Int32(intLiteral: 2000000000);
            let two = std.num.Int32(intLiteral: 2);
            let mulOverflow = bigVal.multiplyChecked(two);
            if mulOverflow.isSome() { return 7 }

            // multiplyChecked — normal case
            let ten = std.num.Int32(intLiteral: 10);
            let five = std.num.Int32(intLiteral: 5);
            let mulNormal = ten.multiplyChecked(five);
            if mulNormal.isNone() { return 8 }
            if mulNormal.unwrap() != std.num.Int32(intLiteral: 50) { return 9 }

            // negateChecked — overflow at -2147483648
            let negMin = minVal.negateChecked();
            if negMin.isSome() { return 10 }

            // negateChecked — normal case
            let negThousand = thousand.negateChecked();
            if negThousand.isNone() { return 11 }
            if negThousand.unwrap() != std.num.Int32(intLiteral: -1000) { return 12 }

            // absChecked — overflow at -2147483648
            let absMin = minVal.absChecked();
            if absMin.isSome() { return 13 }

            // absChecked — normal case
            let negFiveHundred = std.num.Int32(intLiteral: -500);
            let absFiveHundred = negFiveHundred.absChecked();
            if absFiveHundred.isNone() { return 14 }
            if absFiveHundred.unwrap() != fiveHundred { return 15 }

            // addSaturating — clamps to 2147483647
            let addSat = maxVal.addSaturating(one);
            if addSat != maxVal { return 16 }
            let addSatBig = maxVal.addSaturating(std.num.Int32(intLiteral: 100000));
            if addSatBig != maxVal { return 17 }

            // addSaturating — clamps to -2147483648
            let addSatNeg = minVal.addSaturating(negOne);
            if addSatNeg != minVal { return 18 }

            // subtractSaturating — clamps to -2147483648
            let subSat = minVal.subtractSaturating(one);
            if subSat != minVal { return 19 }

            // subtractSaturating — clamps to 2147483647
            let subSatPos = maxVal.subtractSaturating(negOne);
            if subSatPos != maxVal { return 20 }

            // multiplySaturating — clamps to 2147483647
            let mulSat = bigVal.multiplySaturating(two);
            if mulSat != maxVal { return 21 }

            // multiplySaturating — clamps to -2147483648 (positive * negative overflow)
            let negBigVal = std.num.Int32(intLiteral: -2000000000);
            let mulSatNeg = negBigVal.multiplySaturating(two);
            if mulSatNeg != minVal { return 22 }

            // negateSaturating — -2147483648 saturates to 2147483647
            let negSatMin = minVal.negateSaturating();
            if negSatMin != maxVal { return 23 }

            // negateSaturating — normal case
            let negSatThousand = thousand.negateSaturating();
            if negSatThousand != std.num.Int32(intLiteral: -1000) { return 24 }

            // absSaturating — -2147483648 saturates to 2147483647
            let absSatMin = minVal.absSaturating();
            if absSatMin != maxVal { return 25 }

            // absSaturating — normal case
            let absSatNeg = negFiveHundred.absSaturating();
            if absSatNeg != fiveHundred { return 26 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails due to unary minus requiring Negatable protocol resolution
#[test]
fn int32_bitwidth_and_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // byteSwapped — 4-byte swap
            // 0x00000001 = 1 -> byte swapped = 0x01000000 = 16777216
            let one = std.num.Int32(intLiteral: 1);
            let oneSwapped = one.byteSwapped;
            let oneSwappedI64 = std.num.Int64(from: oneSwapped);
            if oneSwappedI64 != 16777216 { return 1 }

            // byteSwapped — 0x01020304 = 16909060 -> 0x04030201 = 67305985
            let val = std.num.Int32(intLiteral: 16909060);
            let swapped = val.byteSwapped;
            let swappedI64 = std.num.Int64(from: swapped);
            if swappedI64 != 67305985 { return 2 }

            // leadingZeros — relative to 32-bit width
            if one.leadingZeros != 31 { return 3 }
            let zero = std.num.Int32(intLiteral: 0);
            if zero.leadingZeros != 32 { return 4 }
            // -1 in Int32 is all 1s, so 0 leading zeros
            let negOne = std.num.Int32(intLiteral: -1);
            if negOne.leadingZeros != 0 { return 5 }
            // 65536 = 2^16, so 15 leading zeros (bit 16 is set, bits 17-31 are 0)
            let val65536 = std.num.Int32(intLiteral: 65536);
            if val65536.leadingZeros != 15 { return 6 }

            // rotateLeft — 32-bit rotation
            if one.rotateLeft(by: 1) != std.num.Int32(intLiteral: 2) { return 7 }
            let fortyTwo = std.num.Int32(intLiteral: 42);
            if fortyTwo.rotateLeft(by: 0) != fortyTwo { return 8 }

            // rotateRight — 32-bit rotation
            let two = std.num.Int32(intLiteral: 2);
            if two.rotateRight(by: 1) != one { return 9 }
            if fortyTwo.rotateRight(by: 0) != fortyTwo { return 10 }

            // rotateLeft and rotateRight are inverses
            let testVal = std.num.Int32(intLiteral: 123456);
            if testVal.rotateLeft(by: 11).rotateRight(by: 11) != testVal { return 11 }

            // init(from:) — from Int64
            let i64val: std.num.Int64 = 100000;
            let fromI64 = std.num.Int32(from: i64val);
            if fromI64 != std.num.Int32(intLiteral: 100000) { return 12 }

            // init(from:) — from Int16
            let i16val = std.num.Int16(intLiteral: -500);
            let fromI16 = std.num.Int32(from: i16val);
            if fromI16 != std.num.Int32(intLiteral: -500) { return 13 }

            // init(from:) — from Int8
            let i8val = std.num.Int8(intLiteral: -100);
            let fromI8 = std.num.Int32(from: i8val);
            if fromI8 != std.num.Int32(intLiteral: -100) { return 14 }

            // parse — valid Int32 value
            let parsed = std.num.Int32.parse(string: "100000");
            if parsed.isNone() { return 15 }
            if parsed.unwrap() != std.num.Int32(intLiteral: 100000) { return 16 }

            // parse — negative value
            let parsedNeg = std.num.Int32.parse(string: "-2147483648");
            if parsedNeg.isNone() { return 17 }
            if parsedNeg.unwrap() != std.num.Int32.minValue { return 18 }

            // parse — maxValue
            let parsedMax = std.num.Int32.parse(string: "2147483647");
            if parsedMax.isNone() { return 19 }
            if parsedMax.unwrap() != std.num.Int32.maxValue { return 20 }

            // parse — out-of-range (too large)
            let parsedBig = std.num.Int32.parse(string: "2147483648");
            if parsedBig.isSome() { return 21 }

            // parse — out-of-range (too small)
            let parsedSmall = std.num.Int32.parse(string: "-2147483649");
            if parsedSmall.isSome() { return 22 }

            // parse — invalid string
            let parsedBad = std.num.Int32.parse(string: "xyz");
            if parsedBad.isSome() { return 23 }

            // parse — empty string
            let parsedEmpty = std.num.Int32.parse(string: "");
            if parsedEmpty.isSome() { return 24 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
