use kestrel_test_suite::*;

#[test]
fn uint64_boundaries_and_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // minValue is 0
            let minVal = std.num.UInt64.minValue;
            let lit0: std.num.UInt64 = 0;
            if minVal.equals(lit0) == false { return 1 }

            // maxValue is 18446744073709551615
            let maxVal = std.num.UInt64.maxValue;
            let lit18446744073709551615: std.num.UInt64 = 18446744073709551615;
            if maxVal.equals(lit18446744073709551615) == false { return 2 }

            // bitWidth is 64
            if std.num.UInt64.bitWidth != 64 { return 3 }

            // zero constant
            let z = std.num.UInt64.zero;
            let zeroLit: std.num.UInt64 = 0;
            if z.equals(zeroLit) == false { return 4 }

            // one constant
            let o = std.num.UInt64.one;
            let oneLit: std.num.UInt64 = 1;
            if o.equals(oneLit) == false { return 5 }

            // isZero
            if minVal.isZero == false { return 6 }
            if maxVal.isZero { return 7 }

            // isPositive
            if maxVal.isPositive == false { return 8 }
            if minVal.isPositive { return 9 }

            // isNegative is always false for unsigned
            if minVal.isNegative { return 10 }
            if maxVal.isNegative { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn uint64_overflow_behavior() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let maxVal = std.num.UInt64.maxValue;
            let minVal = std.num.UInt64.minValue;
            let one: std.num.UInt64 = 1;
            let two: std.num.UInt64 = 2;
            let large: std.num.UInt64 = 10000000000000000000;

            // addChecked — normal case
            let addNorm = one.addChecked(two);
            if addNorm.isNone() { return 1 }
            let three: std.num.UInt64 = 3;
            if addNorm.unwrap().equals(three) == false { return 2 }

            // addChecked — overflow at max
            let addOver = maxVal.addChecked(one);
            if addOver.isSome() { return 3 }

            // subtractChecked — normal case
            let subNorm = two.subtractChecked(one);
            if subNorm.isNone() { return 4 }
            if subNorm.unwrap().equals(one) == false { return 5 }

            // subtractChecked — underflow at 0
            let subUnder = minVal.subtractChecked(one);
            if subUnder.isSome() { return 6 }

            // multiplyChecked — normal case
            let mulThree: std.num.UInt64 = 3;
            let mulNorm = two.multiplyChecked(mulThree);
            if mulNorm.isNone() { return 7 }
            let six: std.num.UInt64 = 6;
            if mulNorm.unwrap().equals(six) == false { return 8 }

            // multiplyChecked — overflow near max
            let mulOver = large.multiplyChecked(two);
            if mulOver.isSome() { return 9 }

            // addSaturating — clamps to maxValue
            let hundred: std.num.UInt64 = 100;
            let addSat = maxVal.addSaturating(hundred);
            if addSat.equals(maxVal) == false { return 10 }

            // addSaturating — normal case
            let addSatNorm = one.addSaturating(two);
            let addSatThree: std.num.UInt64 = 3;
            if addSatNorm.equals(addSatThree) == false { return 11 }

            // subtractSaturating — clamps to 0
            let subSat = minVal.subtractSaturating(one);
            if subSat.equals(std.num.UInt64.zero) == false { return 12 }

            // subtractSaturating — normal case
            let subSatNorm = two.subtractSaturating(one);
            if subSatNorm.equals(one) == false { return 13 }

            // multiplySaturating — clamps to maxValue
            let mulSat = large.multiplySaturating(two);
            if mulSat.equals(maxVal) == false { return 14 }

            // multiplySaturating — normal case
            let mulSatThree: std.num.UInt64 = 3;
            let mulSatNorm = two.multiplySaturating(mulSatThree);
            let mulSatSix: std.num.UInt64 = 6;
            if mulSatNorm.equals(mulSatSix) == false { return 15 }

            // Subtraction wrapping behavior: 0 - 1 wraps to maxValue
            let wrapped = minVal.subtract(one);
            if wrapped.equals(maxVal) == false { return 16 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn uint64_bitwidth_and_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // byteSwapped — 8-byte swap
            // 1 as u64 (0x0000000000000001) -> 0x0100000000000000 (72057594037927936)
            let one: std.num.UInt64 = 1;
            let lit72057594037927936: std.num.UInt64 = 72057594037927936;
            if one.byteSwapped.equals(lit72057594037927936) == false { return 1 }

            // byteSwapped — double swap is identity
            let val: std.num.UInt64 = 123456789;
            if val.byteSwapped.byteSwapped.equals(val) == false { return 2 }

            // leadingZeros — relative to 64-bit width
            if one.leadingZeros != 63 { return 3 }

            let zero: std.num.UInt64 = 0;
            if zero.leadingZeros != 64 { return 4 }

            // Value with high bit set: 2^63 = 9223372036854775808
            let highBit: std.num.UInt64 = 9223372036854775808;
            if highBit.leadingZeros != 0 { return 5 }

            // rotateLeft — 64-bit rotation
            // rotateLeft(1, by: 1) = 2
            let rotTwo: std.num.UInt64 = 2;
            if one.rotateLeft(by: 1).equals(rotTwo) == false { return 6 }
            // rotateLeft(highBit, by: 1) = 1 (wraps around from bit 63 to bit 0)
            if highBit.rotateLeft(by: 1).equals(one) == false { return 7 }

            // rotateRight — 64-bit rotation
            // rotateRight(2, by: 1) = 1
            let two: std.num.UInt64 = 2;
            if two.rotateRight(by: 1).equals(one) == false { return 8 }
            // rotateRight(1, by: 1) = highBit (wraps from bit 0 to bit 63)
            if one.rotateRight(by: 1).equals(highBit) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.num.UInt64 = 123456789;
            if testVal.rotateLeft(by: 17).rotateRight(by: 17).equals(testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64Val: std.num.Int64 = 1000000;
            let fromI64 = std.num.UInt64(from: fromI64Val);
            let lit1000000: std.num.UInt64 = 1000000;
            if fromI64.equals(lit1000000) == false { return 11 }

            // init(from:) — from UInt8
            let fromU8Val: std.num.UInt8 = 255;
            let fromU8 = std.num.UInt64(from: fromU8Val);
            let lit255: std.num.UInt64 = 255;
            if fromU8.equals(lit255) == false { return 12 }

            // parse — valid large value
            let parsed = std.num.UInt64.parse( "18446744073709551615");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().equals(std.num.UInt64.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.num.UInt64.parse( "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().equals(std.num.UInt64.zero) == false { return 16 }

            // parse — out of range (18446744073709551616 > max)
            let parsedOver = std.num.UInt64.parse( "18446744073709551616");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.num.UInt64.parse( "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.num.UInt64.parse( "");
            if parsedEmpty.isSome() { return 19 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
